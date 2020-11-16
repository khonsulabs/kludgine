use super::{Component, InteractiveComponent};
use crate::{
    ui::{Context, Entity},
    KludgineResult,
};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum ListCommand {}

#[derive(Debug, Clone)]
pub enum ListMessage {}

#[derive(Debug, Clone)]
pub enum ListEvent {}

#[async_trait]
pub trait ListRow<T>: InteractiveComponent + Default {
    async fn load(initial_element: T) -> KludgineResult<Self>;
}

#[async_trait]
pub trait Datasource: Send + Sync + 'static {
    type Element: Send + Sync;

    async fn elements(&self) -> KludgineResult<Vec<Self::Element>>;
}

#[async_trait]
pub trait ListDatasource: Datasource {
    type Row: ListRow<Self::Element>;

    async fn row(&self, element: Self::Element) -> KludgineResult<Self::Row>;
}

// TODO It seems like Grid can't be used here. Let's start out tomorrow creating a thorough UI example with multiple windows and transitions between multiple test scenes
// For the list, let's explore the idea of having a Rows<> component which can take the Row type. The List component then
// New idea: If we make a "List" be Scroll<Rows<Row>> the only thing we're missing is data-source handling. But rows could be powered with a data source..
// so yeah, there's really no reason for a List...
// So then do we need to be able to propogate type information up? Yes, we do. That's why we need to be able to have Rows know the child type.
// Grid is really useful for just laying out a bunch of components that you don't need to interact with, like labels.
pub struct List<D>
where
    D: ListDatasource,
{
    datasource: D,
    rows: Option<Vec<Entity<D::Row>>>,
}

impl<D> List<D>
where
    D: ListDatasource,
{
    pub fn new(datasource: D) -> Self {
        Self {
            datasource,
            rows: None,
        }
    }
}

#[async_trait]
impl<D> Component for List<D>
where
    D: ListDatasource,
{
    async fn update(&mut self, context: &mut Context) -> KludgineResult<()> {
        if self.rows.is_none() {
            let elements = self.datasource.elements().await?;
            let mut rows = Vec::new();
            for element in elements {
                rows.push(
                    self.new_entity(context, self.datasource.row(element).await?)
                        .await
                        .insert()
                        .await?,
                );
            }
            self.rows = Some(rows);
        }

        Ok(())
    }
}

#[async_trait]
impl<D> InteractiveComponent for List<D>
where
    D: ListDatasource,
{
    type Message = ListMessage;
    type Command = ListCommand;
    type Event = ListEvent;
}
