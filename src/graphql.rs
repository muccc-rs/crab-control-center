#[derive(Default, Debug)]
pub struct Context {
    pub logic_image: crate::logic::Logic,
}

impl juniper::Context for Context {}

pub type Schema = juniper::RootNode<
    'static,
    Query,
    juniper::EmptyMutation<Context>,
    juniper::EmptySubscription<Context>,
>;

pub fn schema() -> Schema {
    Schema::new(
        Query,
        juniper::EmptyMutation::new(),
        juniper::EmptySubscription::new(),
    )
}

pub struct Query;

#[juniper::graphql_object(Context = Context)]
impl Query {
    fn inputs(context: &Context) -> crate::logic::LogicInputs {
        context.logic_image.inputs().clone()
    }

    fn outputs(context: &Context) -> crate::logic::LogicOutputs {
        context.logic_image.outputs().clone()
    }

    fn state(context: &Context) -> crate::logic::Logic {
        context.logic_image.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema() {
        let schema = juniper::RootNode::new(
            Query,
            juniper::EmptyMutation::<()>::new(),
            juniper::EmptySubscription::<()>::new(),
        );

        println!("{}", schema.as_sdl());

        todo!()
    }
}
