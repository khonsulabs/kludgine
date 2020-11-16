use std::{collections::HashMap, fmt::Debug, hash::Hash};

use super::{
    pending::{AnonymousPendingComponent, PendingComponent},
    InteractiveComponent, InteractiveComponentExt,
};
use crate::{
    math::{Dimension, Scaled, Size, SizeExt},
    style::theme::Selector,
    ui::{
        component::Component, ColumnLayout, Context, Entity, Indexable, LayoutSolver,
        LayoutSolverExt, RowLayout, StyledContext,
    },
    KludgineResult,
};
use async_trait::async_trait;
use generational_arena::Index;

enum GridContents<K, T> {
    AnonymousPending(Vec<(K, Box<dyn AnonymousPendingComponent>, Dimension)>),
    Pending(Vec<(K, PendingComponent<T>, Dimension)>),
    Children(GridCells<K>),
}

#[derive(Debug)]
struct GridCells<K> {
    entities: HashMap<K, Index>,
    dimensions: HashMap<Index, Dimension>,
    ordered: Vec<Index>,
}

impl<K> GridCells<K>
where
    K: Hash + Eq + Debug + Clone + Send + Sync + 'static,
{
    fn push(&mut self, key: K, index: Index, dimension: Dimension) {
        self.entities.insert(key, index);
        self.dimensions.insert(index, dimension);
        self.ordered.push(index);
    }
}

impl<K> Default for GridCells<K> {
    fn default() -> Self {
        Self {
            entities: Default::default(),
            dimensions: Default::default(),
            ordered: Default::default(),
        }
    }
}

pub struct Grid<K, T> {
    horizontal: bool,
    contents: GridContents<K, T>,
}

pub struct MixedGridContentsBuilder {
    grid: Grid<(), ()>,
}

pub struct GridContentsBuilder<K, T> {
    grid: Grid<K, T>,
}

impl Grid<(), ()> {
    pub fn mixed_rows() -> MixedGridContentsBuilder {
        MixedGridContentsBuilder {
            grid: Self {
                horizontal: false,
                contents: GridContents::AnonymousPending(Vec::new()),
            },
        }
    }

    pub fn mixed_columns() -> MixedGridContentsBuilder {
        MixedGridContentsBuilder {
            grid: Self {
                horizontal: true,
                contents: GridContents::AnonymousPending(Vec::new()),
            },
        }
    }
}

impl MixedGridContentsBuilder {
    pub fn cell<T: InteractiveComponent + 'static>(
        mut self,
        cell: T,
        dimension: Dimension,
    ) -> Self {
        if let GridContents::AnonymousPending(contents) = &mut self.grid.contents {
            contents.push(((), Box::new(PendingComponent::Pending(cell)), dimension));
        } else {
            unreachable!()
        }

        self
    }

    pub fn build(self) -> Grid<(), ()> {
        self.grid
    }
}

impl<K, T> GridContentsBuilder<K, T> {
    pub fn cell(mut self, key: K, cell: T, dimension: Dimension) -> Self {
        if let GridContents::Pending(contents) = &mut self.grid.contents {
            contents.push((key, PendingComponent::Pending(cell), dimension));
        } else {
            unreachable!()
        }

        self
    }

    pub fn build(self) -> Grid<K, T> {
        self.grid
    }
}

impl<K, T> Grid<K, T> {
    pub fn rows() -> GridContentsBuilder<K, T> {
        GridContentsBuilder {
            grid: Self {
                horizontal: false,
                contents: GridContents::Pending(Vec::new()),
            },
        }
    }

    pub fn columns() -> GridContentsBuilder<K, T> {
        GridContentsBuilder {
            grid: Self {
                horizontal: true,
                contents: GridContents::Pending(Vec::new()),
            },
        }
    }
}

#[async_trait]
impl<K, T> Component for Grid<K, T>
where
    K: Hash + Eq + Debug + Clone + Send + Sync + 'static,
    T: InteractiveComponent + Send + Sync + 'static,
{
    fn classes(&self) -> Option<Vec<Selector>> {
        Some(vec![Selector::from("grid")])
    }

    async fn initialize(&mut self, context: &mut Context) -> KludgineResult<()> {
        match std::mem::replace(
            &mut self.contents,
            GridContents::Children(Default::default()),
        ) {
            GridContents::AnonymousPending(contents) => {
                let mut cells = GridCells::default();
                for (key, mut pending_child, dimension) in contents {
                    let index = pending_child.insert(context).await?;
                    cells.push(key, index, dimension);
                }
                self.contents = GridContents::Children(cells);
            }
            GridContents::Pending(contents) => {
                let mut children = GridCells::default();
                for (key, pending_child, dimension) in contents {
                    if let PendingComponent::Pending(component) = pending_child {
                        children.push(
                            key.clone(),
                            self.new_entity(context, component)
                                .await
                                .callback(&self.entity(context), move |event| {
                                    GridMessage::ChildEvent(GridEvent {
                                        key: key.clone(),
                                        event,
                                    })
                                })
                                .insert()
                                .await?
                                .index(),
                            dimension,
                        );
                    } else {
                        unreachable!()
                    }
                }
                self.contents = GridContents::Children(children);
            }
            GridContents::Children(_) => unreachable!("A component should never be re-initialized"),
        }

        Ok(())
    }

    async fn content_size(
        &self,
        context: &mut StyledContext,
        constraints: &Size<Option<f32>, Scaled>,
    ) -> KludgineResult<Size<f32, Scaled>> {
        let mut total_size = Size::default();

        if let GridContents::Children(children) = &self.contents {
            for child in children.ordered.iter() {
                let (content_size, padding) = context
                    .content_size_with_padding(child, constraints)
                    .await?;
                let child_size = content_size + padding.minimum_size();

                if self.horizontal {
                    total_size = Size::from_lengths(
                        total_size.width() + child_size.width(),
                        total_size.height().max(child_size.height()),
                    )
                } else {
                    total_size = Size::from_lengths(
                        total_size.width().max(child_size.width()),
                        total_size.height() + child_size.height(),
                    )
                }
            }
        }

        Ok(total_size)
    }

    async fn layout(
        &mut self,
        _context: &mut StyledContext,
    ) -> KludgineResult<Box<dyn LayoutSolver>> {
        if let GridContents::Children(children) = &self.contents {
            if self.horizontal {
                let mut layout = ColumnLayout::default();

                for child in children.ordered.iter() {
                    layout = layout.column(*child, children.dimensions[child]);
                }

                layout.layout()
            } else {
                let mut layout = RowLayout::default();

                for child in children.ordered.iter() {
                    layout = layout.row(*child, children.dimensions[child]);
                }

                layout.layout()
            }
        } else {
            unreachable!()
        }
    }
}

#[derive(Clone, Debug)]
pub enum GridMessage<K, E> {
    ChildEvent(GridEvent<K, E>),
}

#[derive(Clone, Debug)]
pub struct GridEvent<K, E> {
    pub key: K,
    pub event: E,
}

#[derive(Clone, Debug)]
pub struct GridCommand<K, C> {
    pub key: K,
    pub command: C,
}

#[async_trait]
impl<K, T> InteractiveComponent for Grid<K, T>
where
    K: Hash + Eq + Debug + Clone + Send + Sync + 'static,
    T: InteractiveComponent + Send + Sync + 'static,
{
    type Message = GridMessage<K, T::Event>;

    type Command = GridCommand<K, T::Command>;

    type Event = GridEvent<K, T::Event>;

    async fn receive_message(
        &mut self,
        context: &mut Context,
        message: Self::Message,
    ) -> KludgineResult<()> {
        let GridMessage::ChildEvent(event) = message;
        self.callback(context, event).await;
        Ok(())
    }

    async fn receive_command(
        &mut self,
        context: &mut Context,
        command: Self::Command,
    ) -> KludgineResult<()> {
        if let GridContents::Children(children) = &self.contents {
            if let Some(index) = children.entities.get(&command.key) {
                let entity = Entity::<T>::new(context.clone_for(index));
                entity.send(command.command).await?;
            }
        }
        Ok(())
    }
}
