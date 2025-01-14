// Fallback values when no fieldbus connection is available
cfg_if::cfg_if! {
    if #[cfg(feature = "fieldbus")] {
        use crate::fieldbus::PII_SIZE;
        use crate::fieldbus::PIQ_SIZE;
    } else {
        const PIQ_SIZE: usize = 16;
        const PII_SIZE: usize = 16;
    }
}

#[derive(Debug)]
pub struct Context {
    pub logic_image: crate::logic::Logic,
    pub pii: [u8; PII_SIZE],
    pub piq: [u8; PIQ_SIZE],
    pub now: std::time::Instant,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            logic_image: Default::default(),
            pii: [0u8; PII_SIZE],
            piq: [0u8; PIQ_SIZE],
            now: std::time::Instant::now(),
        }
    }
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

    fn hardware_inputs<'a>(context: &'a Context) -> ProcessImage<'a> {
        ProcessImage {
            process_image: &context.pii[..],
        }
    }

    fn hardware_outputs<'a>(context: &'a Context) -> ProcessImage<'a> {
        ProcessImage {
            process_image: &context.piq[..],
        }
    }
}

pub struct ProcessImage<'a> {
    process_image: &'a [u8],
}

#[juniper::graphql_object]
impl<'a> ProcessImage<'a> {
    fn length(&self) -> juniper::FieldResult<i32> {
        self.process_image.len().try_into().map_err(Into::into)
    }

    fn tag_boolean(&self, addr: i32, bit: i32) -> juniper::FieldResult<bool> {
        let addr = usize::try_from(addr)?;
        let bit = usize::try_from(bit)?;
        if addr >= self.process_image.len() {
            return Err("address too big".into());
        }
        if bit > 7 {
            return Err("bit must be 0..7".into());
        }
        Ok(process_image::tag!(&self.process_image, X, addr, bit))
    }

    fn tag_word(&self, addr: i32) -> juniper::FieldResult<i32> {
        let addr = usize::try_from(addr)?;
        if addr >= self.process_image.len() {
            return Err("address too big".into());
        }
        Ok(process_image::tag!(&self.process_image, W, addr).into())
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
