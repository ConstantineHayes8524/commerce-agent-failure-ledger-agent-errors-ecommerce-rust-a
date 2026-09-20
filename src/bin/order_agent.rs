use commerce_agent_failure_ledger::infrai_errors::InfraiErrorClient;
use commerce_agent_failure_ledger::order_loop::{AgentStepError, CommerceSteps, Order, OrderAgent};

struct DemoStore;

impl CommerceSteps for DemoStore {
    fn checkout(&mut self, _order: &Order) -> Result<(), AgentStepError> {
        Ok(())
    }

    fn fulfill(&mut self, _order: &Order) -> Result<(), AgentStepError> {
        Ok(())
    }

    fn send_receipt(&mut self, _order: &Order) -> Result<(), AgentStepError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent = OrderAgent::new(InfraiErrorClient::from_env()?);
    let order = Order {
        order_id: "ord_1042".to_owned(),
        customer_id: "cus_77".to_owned(),
        total_cents: 12_500,
    };
    let update = agent.run(&order, &mut DemoStore).await?;
    println!("{}", serde_json::to_string_pretty(&update)?);
    Ok(())
}
