struct Query;

#[juniper::graphql_object]
impl Query {
    fn inputs() -> juniper::FieldResult<crate::logic::LogicInputs> {
        todo!()
    }

    fn outputs() -> juniper::FieldResult<crate::logic::LogicOutputs> {
        todo!()
    }

    fn state() -> juniper::FieldResult<crate::logic::Logic> {
        todo!()
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
