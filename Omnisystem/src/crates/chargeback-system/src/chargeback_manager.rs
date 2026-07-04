use crate::{CostAllocation, Invoice, LineItem, ChargebackResult, ChargebackError, CostAllocationRule, AllocationMethod, ChargebackStatement};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ChargebackManager {
    allocations: Arc<DashMap<Uuid, CostAllocation>>,
    invoices: Arc<DashMap<Uuid, Invoice>>,
    rules: Arc<DashMap<Uuid, CostAllocationRule>>,
}

impl ChargebackManager {
    pub fn new() -> Self {
        Self {
            allocations: Arc::new(DashMap::new()),
            invoices: Arc::new(DashMap::new()),
            rules: Arc::new(DashMap::new()),
        }
    }

    pub async fn allocate_cost(
        &self,
        parent_id: &str,
        child_id: &str,
        amount: f64,
        percentage: f32,
    ) -> ChargebackResult<Uuid> {
        if percentage < 0.0 || percentage > 100.0 {
            return Err(ChargebackError::InvalidAllocationPercentage);
        }

        let allocation = CostAllocation {
            allocation_id: Uuid::new_v4(),
            parent_id: parent_id.to_string(),
            child_id: child_id.to_string(),
            allocation_percent: percentage,
            amount,
        };

        let id = allocation.allocation_id;
        self.allocations.insert(id, allocation);
        Ok(id)
    }

    pub async fn generate_invoice(
        &self,
        tenant_id: &str,
        total_amount: f64,
        line_items: Vec<LineItem>,
    ) -> ChargebackResult<Uuid> {
        let invoice_id = Uuid::new_v4();
        let invoice = Invoice {
            invoice_id,
            tenant_id: tenant_id.to_string(),
            billing_period: format!("{}", Utc::now().format("%Y-%m")),
            total_amount,
            line_items,
            generated_at: Utc::now(),
            due_date: Utc::now() + chrono::Duration::days(30),
            status: crate::InvoiceStatus::Issued,
        };

        self.invoices.insert(invoice_id, invoice);
        Ok(invoice_id)
    }

    pub async fn get_invoice(&self, invoice_id: Uuid) -> ChargebackResult<Invoice> {
        self.invoices
            .get(&invoice_id)
            .map(|i| i.clone())
            .ok_or(ChargebackError::InvoiceGenerationFailed)
    }

    pub async fn mark_invoice_paid(&self, invoice_id: Uuid) -> ChargebackResult<()> {
        if let Some(mut invoice) = self.invoices.get_mut(&invoice_id) {
            invoice.status = crate::InvoiceStatus::Paid;
            Ok(())
        } else {
            Err(ChargebackError::InvoiceGenerationFailed)
        }
    }

    pub async fn create_allocation_rule(
        &self,
        name: &str,
        method: AllocationMethod,
    ) -> ChargebackResult<Uuid> {
        let rule = CostAllocationRule {
            rule_id: Uuid::new_v4(),
            name: name.to_string(),
            allocation_method: method,
            active: true,
        };

        let id = rule.rule_id;
        self.rules.insert(id, rule);
        Ok(id)
    }

    pub async fn generate_statement(
        &self,
        tenant_id: &str,
        total_cost: f64,
    ) -> ChargebackResult<ChargebackStatement> {
        let mut allocated_costs = Vec::new();

        for entry in self.allocations.iter() {
            let alloc = entry.value();
            if alloc.parent_id == tenant_id {
                allocated_costs.push((alloc.child_id.clone(), alloc.amount));
            }
        }

        Ok(ChargebackStatement {
            statement_id: Uuid::new_v4(),
            tenant_id: tenant_id.to_string(),
            period_start: Utc::now() - chrono::Duration::days(30),
            period_end: Utc::now(),
            total_cost,
            allocated_costs,
            generated_at: Utc::now(),
        })
    }

    pub fn allocation_count(&self) -> usize {
        self.allocations.len()
    }
}

impl Default for ChargebackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_allocate_cost() {
        let manager = ChargebackManager::new();
        let result = manager.allocate_cost("dept1", "team1", 1000.0, 50.0).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_allocation_percentage() {
        let manager = ChargebackManager::new();
        let result = manager.allocate_cost("dept1", "team1", 1000.0, 150.0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_invoice() {
        let manager = ChargebackManager::new();
        let items = vec![
            LineItem {
                item_id: Uuid::new_v4(),
                description: "Compute".to_string(),
                quantity: 10.0,
                unit_price: 50.0,
                total: 500.0,
            },
        ];

        let result = manager.generate_invoice("tenant1", 500.0, items).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mark_invoice_paid() {
        let manager = ChargebackManager::new();
        let items = vec![];
        let invoice_id = manager.generate_invoice("tenant1", 1000.0, items).await.unwrap();

        let result = manager.mark_invoice_paid(invoice_id).await;
        assert!(result.is_ok());

        let invoice = manager.get_invoice(invoice_id).await.unwrap();
        assert_eq!(invoice.status, crate::InvoiceStatus::Paid);
    }
}
