use bioma_actor::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BehaviorTick;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum BehaviorStatus {
    Running,
    Success,
    Failure,
}

pub trait Behavior: Sized {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActionBehavior<B: Behavior> {
    node: B,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DecoratorBehavior<B: Behavior> {
    node: B,
    child: ActorId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompositeBehavior<B: Behavior> {
    node: B,
    children: Vec<ActorId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use tracing::info;

    // MockAction behavior
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockAction {
        fact: String,
    }

    impl Behavior for MockAction {}

    impl Message<BehaviorTick> for ActionBehavior<MockAction> {
        type Response = BehaviorStatus;

        async fn handle(
            &mut self,
            ctx: &mut ActorContext<Self>,
            _msg: &BehaviorTick,
        ) -> Result<BehaviorStatus, Self::Error> {
            info!("{} {}", ctx.id(), self.node.fact);
            Ok(BehaviorStatus::Success)
        }
    }

    impl Actor for ActionBehavior<MockAction> {
        type Error = SystemActorError;

        async fn start(&mut self, ctx: &mut ActorContext<Self>) -> Result<(), Self::Error> {
            let mut stream = ctx.recv().await?;
            while let Some(Ok(frame)) = stream.next().await {
                if let Some(BehaviorTick) = frame.is::<BehaviorTick>() {
                    self.reply(ctx, &BehaviorTick, &frame).await?;
                }
            }
            Ok(())
        }
    }

    // MockDecorator behavior
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockDecorator;

    impl Behavior for MockDecorator {}

    impl Message<BehaviorTick> for DecoratorBehavior<MockDecorator> {
        type Response = BehaviorStatus;

        async fn handle(
            &mut self,
            ctx: &mut ActorContext<Self>,
            _msg: &BehaviorTick,
        ) -> Result<BehaviorStatus, Self::Error> {
            let status: BehaviorStatus = ctx.send_as(BehaviorTick, &self.child, SendOptions::default()).await?;
            Ok(status)
        }
    }

    impl Actor for DecoratorBehavior<MockDecorator> {
        type Error = SystemActorError;

        async fn start(&mut self, ctx: &mut ActorContext<Self>) -> Result<(), Self::Error> {
            let mut stream = ctx.recv().await?;
            while let Some(Ok(frame)) = stream.next().await {
                if let Some(BehaviorTick) = frame.is::<BehaviorTick>() {
                    self.reply(ctx, &BehaviorTick, &frame).await?;
                }
            }
            Ok(())
        }
    }

    // MockComposite behavior
    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockComposite;

    impl Behavior for MockComposite {}

    impl Message<BehaviorTick> for CompositeBehavior<MockComposite> {
        type Response = BehaviorStatus;

        async fn handle(
            &mut self,
            ctx: &mut ActorContext<Self>,
            _msg: &BehaviorTick,
        ) -> Result<BehaviorStatus, Self::Error> {
            for child in &self.children {
                let status: BehaviorStatus = ctx.send_as(BehaviorTick, child, SendOptions::default()).await?;
                info!("{} {} {:?}", ctx.id(), child, status);
            }
            Ok(BehaviorStatus::Success)
        }
    }

    impl Actor for CompositeBehavior<MockComposite> {
        type Error = SystemActorError;

        async fn start(&mut self, ctx: &mut ActorContext<Self>) -> Result<(), Self::Error> {
            let mut stream = ctx.recv().await?;
            while let Some(Ok(frame)) = stream.next().await {
                if let Some(BehaviorTick) = frame.is::<BehaviorTick>() {
                    self.reply(ctx, &BehaviorTick, &frame).await?;
                }
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MainActor {
        root: ActorId,
    }

    impl Actor for MainActor {
        type Error = SystemActorError;

        async fn start(&mut self, ctx: &mut ActorContext<Self>) -> Result<(), Self::Error> {
            let status: BehaviorStatus = ctx.send_as(BehaviorTick, &self.root, SendOptions::default()).await?;
            info!("{} {:?}", ctx.id(), status);
            Ok(())
        }
    }

    #[test(tokio::test)]
    async fn test_behavior_mock() -> Result<(), SystemActorError> {
        // Initialize the engine
        let engine = Engine::test().await?;

        let mock_action_id_0 = ActorId::of::<ActionBehavior<MockAction>>("mock_action_0");
        let mock_action_id_1 = ActorId::of::<ActionBehavior<MockAction>>("mock_action_1");
        let mock_action_id_2 = ActorId::of::<ActionBehavior<MockAction>>("mock_action_2");
        let mock_action_id_3 = ActorId::of::<ActionBehavior<MockAction>>("mock_action_3");

        let mock_decorator_id = ActorId::of::<DecoratorBehavior<MockDecorator>>("mock_decorator");

        let mock_composite_id = ActorId::of::<CompositeBehavior<MockComposite>>("mock_composite");

        let mock_action_0 = ActionBehavior::<MockAction> { node: MockAction { fact: "0".to_string() } };
        let mock_action_1 = ActionBehavior::<MockAction> { node: MockAction { fact: "1".to_string() } };
        let mock_action_2 = ActionBehavior::<MockAction> { node: MockAction { fact: "2".to_string() } };
        let mock_action_3 = ActionBehavior::<MockAction> { node: MockAction { fact: "3".to_string() } };

        let mock_composite = CompositeBehavior::<MockComposite> {
            node: MockComposite,
            children: vec![
                mock_action_id_0.clone(),
                mock_action_id_1.clone(),
                mock_action_id_2.clone(),
                mock_action_id_3.clone(),
            ],
        };

        let mock_decorator =
            DecoratorBehavior::<MockDecorator> { node: MockDecorator, child: mock_composite_id.clone() };

        // Spawn the actors
        let (mut mock_action_0_ctx, mut mock_action_0) =
            Actor::spawn(engine.clone(), mock_action_id_0.clone(), mock_action_0, SpawnOptions::default()).await?;
        let (mut mock_action_1_ctx, mut mock_action_1) =
            Actor::spawn(engine.clone(), mock_action_id_1.clone(), mock_action_1, SpawnOptions::default()).await?;
        let (mut mock_action_2_ctx, mut mock_action_2) =
            Actor::spawn(engine.clone(), mock_action_id_2.clone(), mock_action_2, SpawnOptions::default()).await?;
        let (mut mock_action_3_ctx, mut mock_action_3) =
            Actor::spawn(engine.clone(), mock_action_id_3.clone(), mock_action_3, SpawnOptions::default()).await?;
        let (mut mock_composite_ctx, mut mock_composite) =
            Actor::spawn(engine.clone(), mock_composite_id.clone(), mock_composite, SpawnOptions::default()).await?;
        let (mut mock_decorator_ctx, mut mock_decorator) =
            Actor::spawn(engine.clone(), mock_decorator_id.clone(), mock_decorator, SpawnOptions::default()).await?;

        // Start the actors
        tokio::spawn(async move { mock_action_0.start(&mut mock_action_0_ctx).await.unwrap() });
        tokio::spawn(async move { mock_action_1.start(&mut mock_action_1_ctx).await.unwrap() });
        tokio::spawn(async move { mock_action_2.start(&mut mock_action_2_ctx).await.unwrap() });
        tokio::spawn(async move { mock_action_3.start(&mut mock_action_3_ctx).await.unwrap() });
        tokio::spawn(async move { mock_composite.start(&mut mock_composite_ctx).await.unwrap() });
        tokio::spawn(async move { mock_decorator.start(&mut mock_decorator_ctx).await.unwrap() });

        tokio::time::sleep(std::time::Duration::from_secs(0)).await;

        // Main actor
        let main_actor_id = ActorId::of::<MainActor>("main");
        let main_actor = MainActor { root: mock_decorator_id };
        let (mut main_ctx, mut main_actor) =
            Actor::spawn(engine.clone(), main_actor_id.clone(), main_actor, SpawnOptions::default()).await?;
        main_actor.start(&mut main_ctx).await?;

        // Export the database for debugging
        dbg_export_db!(engine);

        Ok(())
    }
}
