mod arena;
mod component;
mod context;
mod image;
mod label;
mod node;
mod stretch;
pub(crate) use self::{component::BaseComponent, node::NodeData};
pub use self::{
    component::{Component, LayoutConstraints},
    context::*,
    image::Image,
    label::Label,
    node::Node,
};
use crate::{
    math::{Dimension, Rect, Size, Surround},
    runtime::Runtime,
    scene::SceneTarget,
    style::{AlignContent, JustifyContent, Layout, Style},
    ui::stretch::AsyncStretch,
    window::InputEvent,
    KludgineResult,
};
use arena::{HierarchicalArena, Index};
use once_cell::sync::OnceCell;
use std::collections::{HashMap, VecDeque};

static UI: OnceCell<HierarchicalArena> = OnceCell::new();

pub(crate) fn global_arena() -> &'static HierarchicalArena {
    UI.get_or_init(HierarchicalArena::default)
}

pub struct UserInterface<C>
where
    C: Component + 'static,
{
    pub(crate) root: Entity<C>,
    stretch: AsyncStretch,
}

impl<C> UserInterface<C>
where
    C: Component + 'static,
{
    pub async fn new(root: C) -> KludgineResult<Self> {
        let root = Entity::new({
            let node = Node::new(
                root,
                Style::default(),
                Layout {
                    margin: Surround::uniform(Dimension::Points(0.)),
                    size: Size::new(Dimension::Auto, Dimension::Auto),
                    align_content: AlignContent::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
            );

            global_arena().insert(None, node).await
        });

        let ui = Self {
            root,
            stretch: AsyncStretch::default(),
        };
        ui.initialize(root).await?;
        Ok(ui)
    }

    pub async fn render(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        let mut effective_styles = HashMap::new();

        let layouts = {
            let mut computed_styles = HashMap::new();
            let mut traverser = global_arena().traverse(self.root).await;
            let mut found_nodes = VecDeque::new();
            while let Some(index) = traverser.next().await {
                let node_style = global_arena()
                    .get(index)
                    .await
                    .as_ref()
                    .unwrap()
                    .style()
                    .await;
                let computed_style = match global_arena().parent(index).await {
                    Some(parent_index) => {
                        node_style.inherit_from(computed_styles.get(&parent_index).unwrap())
                    }
                    None => node_style.clone(),
                };
                computed_styles.insert(index, computed_style);
                found_nodes.push_back(index);
            }

            for (index, style) in computed_styles {
                effective_styles.insert(index, style.effective_style(scene).await);
            }

            // Traverse the found nodes starting at the back (leaf nodes) and iterate upwards to update stretch
            while let Some(index) = found_nodes.pop_back() {
                let node = global_arena().get(index).await.unwrap();
                let layout = node.layout().await;
                let stretch_style = layout.into();
                let children = global_arena().children(&Some(index)).await;

                let scene_for_context = scene.clone();
                let effective_style = effective_styles.get(&index).unwrap().clone();
                if children.is_empty() {
                    self.stretch.update_leaf(
                        index,
                        stretch_style,
                        Box::new(move |size: Size<Option<f32>>| {
                            Runtime::block_on(async {
                                let mut context = StyledContext::new(
                                    index,
                                    scene_for_context.clone(),
                                    effective_style.clone(),
                                );
                                let node = global_arena().get(index).await.unwrap();
                                node.content_size(&mut context, &size).await
                            })
                        }),
                    )?;
                } else {
                    self.stretch.update_node(index, stretch_style, children)?;
                }
            }

            let size = scene.size().await;
            self.stretch.compute(self.root.index, size).await?
        };

        // TODO for rendering we need to iterate starting at Root but use the layout to order the children indexes before queuing them up
        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let node = global_arena().get(index).await.unwrap();
            let mut context = StyledContext::new(
                index,
                scene.clone(),
                effective_styles.get(&index).unwrap().clone(),
            );
            let layout = layouts.get(&index).unwrap();
            let location = Rect::sized(layout.location.into(), layout.size.into());

            node.render_background(&mut context, &location).await?;
            node.render(&mut context, &location).await?;
        }

        Ok(())
    }

    pub async fn update(&mut self, scene: &SceneTarget) -> KludgineResult<()> {
        // Loop twice, once to allow all the pending messages to be exhausted across all
        // nodes. Then after all messages have been processed, trigger the update method
        // for each node.

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index);
            let node = global_arena().get(index).await.unwrap();

            node.process_pending_events(&mut context).await?;
        }

        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = SceneContext::new(index, scene.clone());
            let node = global_arena().get(index).await.unwrap();

            node.update(&mut context).await?;
        }

        Ok(())
    }

    pub async fn process_input(&mut self, event: InputEvent) -> KludgineResult<()> {
        let mut traverser = global_arena().traverse(self.root).await;
        while let Some(index) = traverser.next().await {
            let mut context = Context::new(index);
            let node = global_arena().get(index).await.unwrap();

            node.process_input(&mut context, event).await?;
        }

        Ok(())
    }

    async fn initialize(&self, index: impl Into<Index>) -> KludgineResult<()> {
        let index = index.into();
        let node = global_arena().get(index).await.unwrap();

        node.initialize(&mut Context::new(index)).await
    }
}

impl<C> Drop for UserInterface<C>
where
    C: Component + 'static,
{
    fn drop(&mut self) {
        let root = self.root;
        Runtime::spawn(async move {
            global_arena().remove(root).await;
        });
    }
}

#[derive(Debug)]
pub struct Entity<C> {
    index: Index,
    _phantom: std::marker::PhantomData<C>,
}

impl<C> Into<Index> for Entity<C> {
    fn into(self) -> Index {
        self.index
    }
}

impl<C> Entity<C> {
    pub fn new(index: Index) -> Self {
        Self {
            index,
            _phantom: std::marker::PhantomData::default(),
        }
    }
}

impl<C> Clone for Entity<C> {
    fn clone(&self) -> Self {
        Self::new(self.index)
    }
}

impl<C> Copy for Entity<C> {}
