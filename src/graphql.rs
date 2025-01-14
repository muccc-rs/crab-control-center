use futures::StreamExt as _;
use std::time::Duration;

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

#[derive(Default, Debug, Clone)]
pub struct Context {
    pub inner: std::sync::Arc<tokio::sync::RwLock<ContextInner>>,
}

#[derive(Debug)]
pub struct ContextInner {
    pub logic_image: crate::logic::Logic,
    pub pii: [u8; PII_SIZE],
    pub piq: [u8; PIQ_SIZE],
    pub now: std::time::Instant,
}

impl Default for ContextInner {
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

pub type Schema = juniper::RootNode<'static, Query, juniper::EmptyMutation<Context>, Subscription>;

pub fn schema() -> Schema {
    Schema::new(Query, juniper::EmptyMutation::new(), Subscription)
}

pub struct Query;

#[juniper::graphql_object(Context = Context)]
impl Query {
    async fn inputs(context: &Context) -> crate::logic::LogicInputs {
        context.inner.read().await.logic_image.inputs().clone()
    }

    async fn outputs(context: &Context) -> crate::logic::LogicOutputs {
        context.inner.read().await.logic_image.outputs().clone()
    }

    async fn state(context: &Context) -> crate::logic::Logic {
        context.inner.read().await.logic_image.clone()
    }

    async fn hardware_inputs<'a>(context: &'a Context) -> ProcessImage<'a> {
        ProcessImage {
            process_image: tokio::sync::RwLockReadGuard::map(context.inner.read().await, |c| {
                &c.pii[..]
            }),
        }
    }

    async fn hardware_outputs<'a>(context: &'a Context) -> ProcessImage<'a> {
        ProcessImage {
            process_image: tokio::sync::RwLockReadGuard::map(context.inner.read().await, |c| {
                &c.piq[..]
            }),
        }
    }
}

pub struct ProcessImage<'a> {
    process_image: tokio::sync::RwLockReadGuard<'a, [u8]>,
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

pub struct Subscription;

type SubscriptionStream<T> =
    std::pin::Pin<Box<dyn futures::stream::Stream<Item = Result<T, juniper::FieldError>> + Send>>;

#[juniper::graphql_subscription(Context = Context)]
impl Subscription {
    async fn watch(period: f64, context: &Context) -> SubscriptionStream<Query> {
        let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Duration::from_secs_f64(period).max(Duration::from_millis(200)),
        ))
        .map(move |_| Ok(Query));
        Box::pin(stream)
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
