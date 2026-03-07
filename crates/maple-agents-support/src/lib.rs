//! MAPLE Reference Customer-Support Agent — ticket triage, escalation, and resolution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── Ticket types ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketStatus {
    Open,
    InProgress,
    WaitingOnCustomer,
    Escalated,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TicketChannel {
    Email,
    Chat,
    Phone,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportTicket {
    pub id: String,
    pub subject: String,
    pub description: String,
    pub customer_id: String,
    pub priority: TicketPriority,
    pub status: TicketStatus,
    pub channel: TicketChannel,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub assigned_to: Option<String>,
}

// ── Agent actions ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportAction {
    Reply { ticket_id: String, message: String },
    Escalate { ticket_id: String, reason: String, to_team: String },
    SetPriority { ticket_id: String, priority: TicketPriority },
    Resolve { ticket_id: String, resolution: String },
    AddTag { ticket_id: String, tag: String },
    Reassign { ticket_id: String, agent_id: String },
}

// ── Escalation rules ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationRule {
    pub name: String,
    pub condition: EscalationCondition,
    pub target_team: String,
    pub auto_escalate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EscalationCondition {
    PriorityAtLeast(TicketPriority),
    AgeExceeds { hours: u32 },
    KeywordMatch { keywords: Vec<String> },
    CustomerTier { tier: String },
}

// ── Errors ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SupportError {
    #[error("ticket not found: {0}")]
    TicketNotFound(String),
    #[error("invalid state transition: {from:?} → {to:?}")]
    InvalidTransition { from: TicketStatus, to: TicketStatus },
    #[error("escalation failed: {0}")]
    EscalationFailed(String),
    #[error("customer not found: {0}")]
    CustomerNotFound(String),
}

// ── Support agent ─────────────────────────────────────────────

pub struct SupportAgent {
    tickets: Vec<SupportTicket>,
    escalation_rules: Vec<EscalationRule>,
}

impl SupportAgent {
    pub fn new() -> Self {
        Self {
            tickets: Vec::new(),
            escalation_rules: Vec::new(),
        }
    }

    pub fn add_escalation_rule(&mut self, rule: EscalationRule) {
        self.escalation_rules.push(rule);
    }

    pub fn create_ticket(
        &mut self,
        subject: &str,
        description: &str,
        customer_id: &str,
        channel: TicketChannel,
    ) -> String {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4().to_string();
        self.tickets.push(SupportTicket {
            id: id.clone(),
            subject: subject.to_string(),
            description: description.to_string(),
            customer_id: customer_id.to_string(),
            priority: TicketPriority::Medium,
            status: TicketStatus::Open,
            channel,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            resolved_at: None,
            assigned_to: None,
        });
        id
    }

    pub fn set_priority(&mut self, ticket_id: &str, priority: TicketPriority) -> Result<(), SupportError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == ticket_id)
            .ok_or_else(|| SupportError::TicketNotFound(ticket_id.to_string()))?;
        ticket.priority = priority;
        ticket.updated_at = Utc::now();
        Ok(())
    }

    pub fn escalate(&mut self, ticket_id: &str, reason: &str) -> Result<(), SupportError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == ticket_id)
            .ok_or_else(|| SupportError::TicketNotFound(ticket_id.to_string()))?;
        if matches!(ticket.status, TicketStatus::Closed | TicketStatus::Resolved) {
            return Err(SupportError::InvalidTransition {
                from: ticket.status.clone(),
                to: TicketStatus::Escalated,
            });
        }
        ticket.status = TicketStatus::Escalated;
        ticket.tags.push(format!("escalated:{}", reason));
        ticket.updated_at = Utc::now();
        Ok(())
    }

    pub fn resolve(&mut self, ticket_id: &str, resolution: &str) -> Result<(), SupportError> {
        let ticket = self
            .tickets
            .iter_mut()
            .find(|t| t.id == ticket_id)
            .ok_or_else(|| SupportError::TicketNotFound(ticket_id.to_string()))?;
        if matches!(ticket.status, TicketStatus::Closed) {
            return Err(SupportError::InvalidTransition {
                from: ticket.status.clone(),
                to: TicketStatus::Resolved,
            });
        }
        ticket.status = TicketStatus::Resolved;
        ticket.tags.push(format!("resolution:{}", resolution));
        ticket.resolved_at = Some(Utc::now());
        ticket.updated_at = Utc::now();
        Ok(())
    }

    pub fn check_escalation_rules(&self, ticket: &SupportTicket) -> Vec<&EscalationRule> {
        self.escalation_rules
            .iter()
            .filter(|rule| match &rule.condition {
                EscalationCondition::PriorityAtLeast(min) => {
                    priority_rank(&ticket.priority) >= priority_rank(min)
                }
                EscalationCondition::KeywordMatch { keywords } => {
                    let text = format!("{} {}", ticket.subject, ticket.description).to_lowercase();
                    keywords.iter().any(|kw| text.contains(&kw.to_lowercase()))
                }
                _ => false,
            })
            .collect()
    }

    pub fn get_tickets(&self) -> &[SupportTicket] {
        &self.tickets
    }

    pub fn get_ticket(&self, id: &str) -> Option<&SupportTicket> {
        self.tickets.iter().find(|t| t.id == id)
    }

    pub fn open_ticket_count(&self) -> usize {
        self.tickets
            .iter()
            .filter(|t| matches!(t.status, TicketStatus::Open | TicketStatus::InProgress | TicketStatus::WaitingOnCustomer))
            .count()
    }
}

impl Default for SupportAgent {
    fn default() -> Self {
        Self::new()
    }
}

fn priority_rank(p: &TicketPriority) -> u8 {
    match p {
        TicketPriority::Low => 0,
        TicketPriority::Medium => 1,
        TicketPriority::High => 2,
        TicketPriority::Urgent => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_ticket() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Help", "I need help", "cust-1", TicketChannel::Email);
        assert!(!id.is_empty());
        assert_eq!(agent.get_tickets().len(), 1);
        assert_eq!(agent.get_ticket(&id).unwrap().status, TicketStatus::Open);
    }

    #[test]
    fn test_set_priority() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Help", "Urgent issue", "cust-1", TicketChannel::Chat);
        agent.set_priority(&id, TicketPriority::Urgent).unwrap();
        assert_eq!(agent.get_ticket(&id).unwrap().priority, TicketPriority::Urgent);
    }

    #[test]
    fn test_escalate_ticket() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Outage", "System down", "cust-1", TicketChannel::Phone);
        agent.escalate(&id, "system-outage").unwrap();
        let ticket = agent.get_ticket(&id).unwrap();
        assert_eq!(ticket.status, TicketStatus::Escalated);
        assert!(ticket.tags.iter().any(|t| t.contains("escalated:")));
    }

    #[test]
    fn test_resolve_ticket() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Bug", "Button broken", "cust-1", TicketChannel::Email);
        agent.resolve(&id, "fixed-in-v2").unwrap();
        let ticket = agent.get_ticket(&id).unwrap();
        assert_eq!(ticket.status, TicketStatus::Resolved);
        assert!(ticket.resolved_at.is_some());
    }

    #[test]
    fn test_cannot_escalate_closed() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Done", "All good", "cust-1", TicketChannel::Api);
        agent.resolve(&id, "done").unwrap();
        // Manually close for test
        agent.tickets[0].status = TicketStatus::Closed;
        assert!(agent.escalate(&id, "reopen").is_err());
    }

    #[test]
    fn test_nonexistent_ticket() {
        let mut agent = SupportAgent::new();
        assert!(agent.set_priority("nope", TicketPriority::High).is_err());
        assert!(agent.escalate("nope", "reason").is_err());
        assert!(agent.resolve("nope", "done").is_err());
    }

    #[test]
    fn test_escalation_rules() {
        let mut agent = SupportAgent::new();
        agent.add_escalation_rule(EscalationRule {
            name: "urgent-escalation".to_string(),
            condition: EscalationCondition::PriorityAtLeast(TicketPriority::High),
            target_team: "tier-2".to_string(),
            auto_escalate: true,
        });
        agent.add_escalation_rule(EscalationRule {
            name: "keyword-escalation".to_string(),
            condition: EscalationCondition::KeywordMatch {
                keywords: vec!["outage".to_string(), "security".to_string()],
            },
            target_team: "incident".to_string(),
            auto_escalate: true,
        });

        let id = agent.create_ticket("Security breach", "Detected intrusion", "cust-1", TicketChannel::Email);
        agent.set_priority(&id, TicketPriority::Urgent).unwrap();
        let ticket = agent.get_ticket(&id).unwrap();
        let matching = agent.check_escalation_rules(ticket);
        assert_eq!(matching.len(), 2);
    }

    #[test]
    fn test_open_ticket_count() {
        let mut agent = SupportAgent::new();
        agent.create_ticket("T1", "desc", "c1", TicketChannel::Email);
        agent.create_ticket("T2", "desc", "c2", TicketChannel::Chat);
        let id3 = agent.create_ticket("T3", "desc", "c3", TicketChannel::Api);
        agent.resolve(&id3, "done").unwrap();
        assert_eq!(agent.open_ticket_count(), 2);
    }

    #[test]
    fn test_ticket_serde() {
        let mut agent = SupportAgent::new();
        let id = agent.create_ticket("Test", "Serde test", "c1", TicketChannel::Email);
        let ticket = agent.get_ticket(&id).unwrap();
        let json = serde_json::to_string(ticket).unwrap();
        let deserialized: SupportTicket = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject, "Test");
        assert_eq!(deserialized.status, TicketStatus::Open);
    }
}
