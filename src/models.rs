use chrono::{Datelike, Duration, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: u64,
    pub description: String,
    /// Positive numbers mean money is leaving. Use negative for income.
    pub amount: f64,
    pub category: String,
    pub date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: u64,
    pub category: String,
    pub monthly_limit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ledger {
    pub transactions: Vec<Transaction>,
    pub budgets: Vec<Budget>,
    pub next_tx_id: u64,
    pub next_budget_id: u64,
}

impl Default for Ledger {
    fn default() -> Self {
        Self::with_sample_data()
    }
}

impl Ledger {
    pub fn with_sample_data() -> Self {
        let now = Local::now().naive_local().date();
        let last_month = now - Duration::days(30);
        let mut ledger = Self {
            transactions: Vec::new(),
            budgets: vec![
                Budget {
                    id: 1,
                    category: "Housing".into(),
                    monthly_limit: 1800.0,
                },
                Budget {
                    id: 2,
                    category: "Food".into(),
                    monthly_limit: 600.0,
                },
                Budget {
                    id: 3,
                    category: "Transport".into(),
                    monthly_limit: 250.0,
                },
            ],
            next_tx_id: 1,
            next_budget_id: 4,
        };

        let sample = vec![
            (
                "Paycheck",
                -4100.0,
                "Income",
                last_month.with_day(27).unwrap(),
            ),
            ("Rent", 1700.0, "Housing", now.with_day(1).unwrap()),
            ("Groceries", 140.0, "Food", now.with_day(3).unwrap()),
            ("Coffee + snacks", 32.5, "Food", now.with_day(4).unwrap()),
            ("Ride share", 24.0, "Transport", now.with_day(5).unwrap()),
            ("Utilities", 220.0, "Housing", now.with_day(7).unwrap()),
            ("Concert night", 120.0, "Fun", now.with_day(10).unwrap()),
            ("Café cowork", 18.5, "Work", now.with_day(12).unwrap()),
            (
                "Savings transfer",
                500.0,
                "Savings",
                now.with_day(15).unwrap(),
            ),
            ("Bonus", -450.0, "Income", now.with_day(16).unwrap()),
            ("Groceries", 90.5, "Food", now.with_day(18).unwrap()),
            ("Gas", 58.0, "Transport", now.with_day(21).unwrap()),
            ("Streaming", 24.0, "Fun", last_month.with_day(16).unwrap()),
        ];

        for (desc, amount, category, date) in sample {
            ledger.add_transaction(desc, amount, category, date);
        }

        ledger
    }

    pub fn add_transaction(
        &mut self,
        description: impl Into<String>,
        amount: f64,
        category: impl Into<String>,
        date: NaiveDate,
    ) {
        let tx = Transaction {
            id: self.next_tx_id,
            description: description.into(),
            amount,
            category: category.into(),
            date,
        };
        self.next_tx_id += 1;
        self.transactions.push(tx);
        self.transactions.sort_by(|a, b| b.date.cmp(&a.date));
    }

    pub fn add_or_update_budget(&mut self, category: impl Into<String>, monthly_limit: f64) {
        let category = category.into();
        if let Some(budget) = self.budgets.iter_mut().find(|b| b.category == category) {
            budget.monthly_limit = monthly_limit;
            return;
        }

        let budget = Budget {
            id: self.next_budget_id,
            category,
            monthly_limit,
        };
        self.next_budget_id += 1;
        self.budgets.push(budget);
    }

    pub fn current_month_overview(&self) -> Overview {
        let now = Local::now().naive_local().date();
        let (income, outgoing) = self.transactions.iter().fold((0.0, 0.0), |mut acc, tx| {
            if tx.date.year() == now.year() && tx.date.month() == now.month() {
                if tx.amount < 0.0 {
                    acc.0 += -tx.amount;
                } else {
                    acc.1 += tx.amount;
                }
            }
            acc
        });

        Overview {
            total_income: income,
            total_outgoing: outgoing,
            net: income - outgoing,
        }
    }

    pub fn category_spending_current_month(&self) -> Vec<(String, f64)> {
        let now = Local::now().naive_local().date();
        let mut by_category: HashMap<String, f64> = HashMap::new();
        for tx in self.transactions.iter().filter(|t| {
            t.amount > 0.0 && t.date.year() == now.year() && t.date.month() == now.month()
        }) {
            *by_category.entry(tx.category.clone()).or_insert(0.0) += tx.amount;
        }

        let mut pairs: Vec<_> = by_category.into_iter().collect();
        pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        pairs
    }

    pub fn budgets_by_category(&self) -> HashMap<String, f64> {
        let mut map = HashMap::new();
        for budget in &self.budgets {
            map.insert(budget.category.clone(), budget.monthly_limit);
        }
        map
    }

    pub fn spending_last_n_months(&self, months: usize) -> Vec<(String, f64)> {
        if months == 0 {
            return Vec::new();
        }
        let mut bucket: HashMap<(i32, u32), f64> = HashMap::new();
        let now = Local::now().naive_local().date();
        let earliest = now - Duration::days((months as i64) * 31);

        for tx in &self.transactions {
            if tx.date < earliest {
                continue;
            }
            let key = (tx.date.year(), tx.date.month());
            // Treat income (negative numbers) as positive inflow.
            *bucket.entry(key).or_insert(0.0) += -tx.amount;
        }

        let mut series: Vec<_> = bucket
            .into_iter()
            .map(|((y, m), v)| (format!("{y}-{m:02}"), v))
            .collect();
        series.sort_by(|a, b| a.0.cmp(&b.0));
        series
    }

    pub fn suggested_budgets(&self) -> Vec<BudgetSuggestion> {
        let cutoff = Local::now().naive_local().date() - Duration::days(90);
        let mut spend: HashMap<String, f64> = HashMap::new();
        for tx in self
            .transactions
            .iter()
            .filter(|t| t.date >= cutoff && t.amount > 0.0)
        {
            *spend.entry(tx.category.clone()).or_insert(0.0) += tx.amount;
        }

        let window_months = 3.0;
        let mut suggestions: Vec<_> = spend
            .into_iter()
            .map(|(cat, amt)| {
                let base = (amt / window_months).max(50.0);
                let suggested = (base * 1.1 * 100.0).round() / 100.0; // 10% buffer
                BudgetSuggestion {
                    category: cat.clone(),
                    suggested_limit: suggested,
                    reason: "Last 90 days average + 10% buffer".to_string(),
                }
            })
            .collect();

        // Encourage common buckets if user does not have any data yet.
        if suggestions.is_empty() {
            suggestions = vec![
                BudgetSuggestion {
                    category: "Housing".into(),
                    suggested_limit: 0.0,
                    reason: "Add your rent/mortgage so you can track it monthly".into(),
                },
                BudgetSuggestion {
                    category: "Food".into(),
                    suggested_limit: 0.0,
                    reason: "Groceries, coffee, restaurants".into(),
                },
                BudgetSuggestion {
                    category: "Savings".into(),
                    suggested_limit: 0.0,
                    reason: "Pay yourself first".into(),
                },
            ];
        }

        suggestions.sort_by(|a, b| {
            b.suggested_limit
                .partial_cmp(&a.suggested_limit)
                .unwrap_or(Ordering::Equal)
        });
        suggestions
    }
}

#[derive(Debug, Clone)]
pub struct Overview {
    pub total_income: f64,
    pub total_outgoing: f64,
    pub net: f64,
}

#[derive(Debug, Clone)]
pub struct BudgetSuggestion {
    pub category: String,
    pub suggested_limit: f64,
    pub reason: String,
}
