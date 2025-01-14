use futures::StreamExt as _;
use std::{pin::Pin, time::Duration};

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

impl Subscription {
    async fn watch<'a>(
        period: f64,
        executor: &juniper::Executor<'_, 'a, Context>,
    ) -> Pin<Box<dyn futures::stream::Stream<Item = juniper::ExecutionResult> + Send + 'a>> {
        let executor = executor.as_owned_executor();
        let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
            Duration::from_secs_f64(period).max(Duration::from_millis(200)),
        ))
        .then(move |_| {
            let executor = executor.clone();
            async move { executor.as_executor().resolve_async(&(), &Query).await }
        })
        .scan(None, |state, q| {
            if let Some(s) = state {
                if s == &q {
                    return std::future::ready(Some(None));
                }
            }
            *state = Some(q.clone());
            std::future::ready(Some(Some(q)))
        })
        .filter_map(|opt| std::future::ready(opt));

        Box::pin(stream)
    }
}

impl juniper::GraphQLType for Subscription {
    fn name(_info: &Self::TypeInfo) -> Option<&str> {
        Some("Subscription")
    }

    fn meta<'r>(
        info: &Self::TypeInfo,
        registry: &mut juniper::Registry<'r>,
    ) -> juniper::meta::MetaType<'r>
    where
        juniper::DefaultScalarValue: 'r,
    {
        let fields = [registry
            .field_convert::<Query, _, Self::Context>("watch", info)
            .argument(registry.arg::<f64>("period", info))];

        registry
            .build_object_type::<Subscription>(info, &fields)
            .into_meta()
    }
}

impl juniper::GraphQLValue for Subscription {
    type Context = Context;
    type TypeInfo = ();

    fn type_name<'i>(&self, info: &'i Self::TypeInfo) -> Option<&'i str> {
        <Self as juniper::GraphQLType>::name(info)
    }
}

impl juniper::GraphQLSubscriptionValue for Subscription {
    fn resolve_field_into_stream<'s, 'i, 'fi, 'args, 'e, 'ref_e, 'res, 'f>(
        &'s self,
        _info: &'i Self::TypeInfo,
        field: &'fi str,
        args: juniper::Arguments<'args>,
        executor: &'ref_e juniper::Executor<'ref_e, 'e, Self::Context>,
    ) -> juniper::BoxFuture<'f, juniper::FieldResult<juniper::Value<juniper::ValuesStream<'res>>>>
    where
        's: 'f,
        'fi: 'f,
        'args: 'f,
        'ref_e: 'f,
        'res: 'f,
        'i: 'res,
        'e: 'res,
    {
        match field {
            "watch" => futures::FutureExt::boxed(async move {
                let stream = Self::watch(
                    args.get::<f64>("period").and_then(|opt| {
                        opt.ok_or_else(|| juniper::FieldError::from("Missing argument `period`"))
                    })?,
                    &executor,
                )
                .await;

                let ex = executor.as_owned_executor();
                let stream = stream.then(move |val| {
                    std::future::ready(val.map_err(|e| ex.as_executor().new_error(e)))
                });

                Ok(juniper::Value::Scalar(stream.boxed()))
            }),
            _ => Box::pin(async move {
                Err(juniper::FieldError::from(format!(
                    "Field `{field}` not found on type `Subscription`",
                )))
            }),
        }
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
