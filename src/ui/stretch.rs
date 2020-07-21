use crate::{math::Size, ui::Index, KludgineError, KludgineResult};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{any::Any, collections::HashMap};
use stretch::{
    geometry::Size as StretchSize,
    node::{Node, Stretch},
    number::Number,
    result::Layout,
    style::Style,
};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub type SafeMeasureFunc =
    Box<dyn Fn(Size<Option<f32>>) -> Result<Size<f32>, KludgineError> + Send + Sync>;

enum StretchCommand {
    UpdateNode {
        index: Index,
        children: Vec<Index>,
        style: Style,
    },
    UpdateLeaf {
        index: Index,
        style: Style,
        measure: SafeMeasureFunc,
    },
    Compute {
        root: Index,
        size: Size<Option<f32>>,
    },
}

pub struct AsyncStretch {
    sender: Sender<StretchCommand>,
    receiver: UnboundedReceiver<KludgineResult<HashMap<Index, Layout>>>,
}

impl AsyncStretch {
    pub fn update_node(
        &self,
        index: Index,
        style: Style,
        children: Vec<Index>,
    ) -> KludgineResult<()> {
        self.sender
            .send(StretchCommand::UpdateNode {
                index,
                style,
                children,
            })
            .map_err(|_| {
                KludgineError::InternalWindowMessageSendError(
                    "error communicating with stretch".into(),
                )
            })
    }

    pub fn update_leaf(
        &self,
        index: Index,
        style: Style,
        measure: SafeMeasureFunc,
    ) -> KludgineResult<()> {
        self.sender
            .send(StretchCommand::UpdateLeaf {
                index,
                style,
                measure,
            })
            .map_err(|_| {
                KludgineError::InternalWindowMessageSendError(
                    "error communicating with stretch".into(),
                )
            })
    }

    pub async fn compute(
        &mut self,
        root: Index,
        size: Size<f32>,
    ) -> KludgineResult<HashMap<Index, Layout>> {
        self.sender
            .send(StretchCommand::Compute {
                root,
                size: Size {
                    width: Some(size.width),
                    height: Some(size.height),
                },
            })
            .map_err(|_| {
                KludgineError::InternalWindowMessageSendError(
                    "error communicating with stretch".into(),
                )
            })?;

        self.receiver.recv().await.unwrap()
    }
}

impl Default for AsyncStretch {
    fn default() -> Self {
        let (command_sender, command_receiver) = unbounded();
        let (results_sender, results_receiver) = unbounded_channel();
        std::thread::Builder::new()
            .name("async-stretch".to_owned())
            .spawn(move || {
                AsyncStretchThread::new(command_receiver, results_sender)
                    .main()
                    .expect("Error on stretch thread")
            })
            .unwrap();
        Self {
            sender: command_sender,
            receiver: results_receiver,
        }
    }
}

struct AsyncStretchThread {
    commands: Receiver<StretchCommand>,
    results: UnboundedSender<KludgineResult<HashMap<Index, Layout>>>,
    nodes: HashMap<Index, Node>,
    stretch: Stretch,
}

impl AsyncStretchThread {
    fn new(
        commands: Receiver<StretchCommand>,
        results: UnboundedSender<KludgineResult<HashMap<Index, Layout>>>,
    ) -> Self {
        Self {
            commands,
            results,
            nodes: HashMap::new(),
            stretch: Stretch::new(),
        }
    }

    fn main(&mut self) -> Result<(), anyhow::Error> {
        while let Ok(command) = self.commands.recv() {
            match command {
                StretchCommand::UpdateNode {
                    index,
                    children,
                    style,
                } => {
                    let children = children
                        .into_iter()
                        .map(|index| *self.nodes.get(&index).unwrap())
                        .collect();

                    if let Some(&node) = self.nodes.get(&index) {
                        self.stretch.set_style(node, style).unwrap();
                        self.stretch.set_children(node, children).unwrap();
                        self.stretch.set_measure(node, None).unwrap();
                    } else {
                        self.nodes
                            .insert(index, self.stretch.new_node(style, children).unwrap());
                    }
                }
                StretchCommand::UpdateLeaf {
                    index,
                    style,
                    measure,
                } => {
                    let measure = Box::new(move |size: StretchSize<Number>| {
                        measure(size.into())
                            .map(|size| size.into())
                            .map_err::<Box<dyn Any>, _>(|err| Box::new(err))
                    });
                    if let Some(&node) = self.nodes.get(&index) {
                        self.stretch.set_style(node, style).unwrap();
                        self.stretch.set_measure(node, Some(measure)).unwrap();
                    } else {
                        self.nodes
                            .insert(index, self.stretch.new_leaf(style, measure).unwrap());
                    }
                }
                StretchCommand::Compute { root, size } => {
                    self.stretch
                        .compute_layout(
                            *self.nodes.get(&root).unwrap(),
                            stretch::geometry::Size {
                                width: size
                                    .width
                                    .map(stretch::number::Number::Defined)
                                    .unwrap_or_default(),
                                height: size
                                    .height
                                    .map(stretch::number::Number::Defined)
                                    .unwrap_or_default(),
                            },
                        )
                        .unwrap();

                    let mut results = HashMap::new();
                    for (&index, &node) in self.nodes.iter() {
                        let layout = self.stretch.layout(node).unwrap();
                        results.insert(index, *layout);
                    }

                    self.results.send(Ok(results))?;
                }
            }
        }

        Ok(())
    }
}
