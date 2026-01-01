//! Agent Coordinator - Multi-Agent Orchestration & Communication
//!
//! Coordinates multiple agents for:
//! - Job distribution
//! - Agent consensus
//! - Message routing
//! - Conflict resolution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::SystemTime;

use super::*;

/// Message types for agent-to-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Request data from another agent
    DataRequest {
        from: AgentId,
        to: AgentId,
        query: String,
    },
    /// Respond with data
    DataResponse {
        from: AgentId,
        to: AgentId,
        data: Vec<u8>,
    },
    /// Propose a collaborative action
    ProposalRequest {
        from: AgentId,
        recipients: Vec<AgentId>,
        proposal: String,
        data: Vec<u8>,
    },
    /// Vote on a proposal
    ProposalVote {
        from: AgentId,
        proposal_id: u64,
        vote: VoteChoice,
    },
    /// Notify about job completion
    JobNotification {
        from: AgentId,
        job_id: JobId,
        status: JobStatus,
    },
    /// Generic alert
    Alert {
        from: AgentId,
        severity: AlertSeverity,
        message: String,
    },
}

/// Vote choice for proposals
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Active proposal for agent consensus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: AgentId,
    pub title: String,
    pub description: String,
    pub recipients: Vec<AgentId>,
    pub votes: HashMap<AgentId, VoteChoice>,
    pub created_at: u64,
    pub expires_at: u64,
}

impl Proposal {
    /// Check if proposal has reached consensus (>50% yes votes)
    pub fn has_consensus(&self) -> bool {
        if self.recipients.is_empty() {
            return false;
        }
        
        let yes_votes = self.votes.values().filter(|v| **v == VoteChoice::Yes).count();
        yes_votes * 2 > self.recipients.len()
    }

    /// Get voting status
    pub fn voting_status(&self) -> ProposalStatus {
        let total = self.recipients.len();
        let votes_cast = self.votes.len();
        let yes_votes = self.votes.values().filter(|v| **v == VoteChoice::Yes).count();
        let no_votes = self.votes.values().filter(|v| **v == VoteChoice::No).count();

        ProposalStatus {
            total_voters: total as u32,
            votes_cast: votes_cast as u32,
            yes_votes: yes_votes as u32,
            no_votes: no_votes as u32,
            consensus_reached: self.has_consensus(),
        }
    }
}

/// Status of a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalStatus {
    pub total_voters: u32,
    pub votes_cast: u32,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub consensus_reached: bool,
}

/// Agent coordinator service
pub struct AgentCoordinator {
    executor: Arc<SwarmExecutor>,
    messages: Arc<RwLock<Vec<Message>>>,
    proposals: Arc<RwLock<HashMap<u64, Proposal>>>,
    proposal_counter: Arc<RwLock<u64>>,
}

impl AgentCoordinator {
    /// Create a new agent coordinator
    pub fn new(executor: Arc<SwarmExecutor>) -> Self {
        Self {
            executor,
            messages: Arc::new(RwLock::new(Vec::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            proposal_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Send a message between agents
    pub async fn send_message(&self, message: Message) -> Result<(), String> {
        let mut messages = self.messages.write().await;
        messages.push(message);
        Ok(())
    }

    /// Get messages for an agent
    pub async fn get_messages(&self, agent_id: &AgentId) -> Result<Vec<Message>, String> {
        let messages = self.messages.read().await;
        Ok(messages
            .iter()
            .filter(|m| {
                match m {
                    Message::DataResponse { to, .. } => to == agent_id,
                    Message::ProposalRequest { recipients, .. } => recipients.contains(agent_id),
                    Message::ProposalVote { from, .. } => from == agent_id,
                    Message::JobNotification { from, .. } => from == agent_id,
                    Message::Alert { from, .. } => from == agent_id,
                    Message::DataRequest { to, .. } => to == agent_id,
                }
            })
            .cloned()
            .collect())
    }

    /// Clear messages for an agent
    pub async fn clear_messages(&self, agent_id: &AgentId) -> Result<(), String> {
        let mut messages = self.messages.write().await;
        messages.retain(|m| {
            match m {
                Message::DataResponse { to, .. } => to != agent_id,
                Message::ProposalRequest { recipients, .. } => !recipients.contains(agent_id),
                Message::JobNotification { from, .. } => from != agent_id,
                Message::Alert { from, .. } => from != agent_id,
                _ => true,
            }
        });
        Ok(())
    }

    /// Create a new proposal for agent consensus
    pub async fn create_proposal(
        &self,
        proposer: AgentId,
        title: String,
        description: String,
        recipients: Vec<AgentId>,
    ) -> Result<u64, String> {
        let mut counter = self.proposal_counter.write().await;
        *counter += 1;
        let proposal_id = *counter;
        drop(counter);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let proposal = Proposal {
            id: proposal_id,
            proposer,
            title,
            description,
            recipients,
            votes: HashMap::new(),
            created_at: now,
            expires_at: now + 3600, // 1 hour expiry
        };

        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal_id, proposal);

        Ok(proposal_id)
    }

    /// Vote on a proposal
    pub async fn vote_on_proposal(
        &self,
        proposal_id: u64,
        voter: AgentId,
        choice: VoteChoice,
    ) -> Result<(), String> {
        let mut proposals = self.proposals.write().await;
        let proposal = proposals
            .get_mut(&proposal_id)
            .ok_or("Proposal not found".to_string())?;

        if !proposal.recipients.contains(&voter) {
            return Err("Not authorized to vote".to_string());
        }

        proposal.votes.insert(voter, choice);
        Ok(())
    }

    /// Get proposal details
    pub async fn get_proposal(&self, proposal_id: u64) -> Result<Option<Proposal>, String> {
        let proposals = self.proposals.read().await;
        Ok(proposals.get(&proposal_id).cloned())
    }

    /// Get proposal status
    pub async fn get_proposal_status(&self, proposal_id: u64) -> Result<Option<ProposalStatus>, String> {
        let proposals = self.proposals.read().await;
        Ok(proposals.get(&proposal_id).map(|p| p.voting_status()))
    }

    /// List active proposals
    pub async fn list_proposals(&self) -> Result<Vec<Proposal>, String> {
        let proposals = self.proposals.read().await;
        Ok(proposals.values().cloned().collect())
    }

    /// Distribute jobs across agents
    pub async fn distribute_jobs(
        &self,
        agents: Vec<AgentId>,
        action: AgentAction,
        priority: JobPriority,
    ) -> Result<Vec<JobId>, String> {
        let mut job_ids = Vec::new();
        
        for agent_id in agents {
            match self.executor.submit_job(agent_id, action.clone(), priority).await {
                Ok(job_id) => job_ids.push(job_id),
                Err(e) => eprintln!("Failed to submit job to agent: {}", e),
            }
        }

        Ok(job_ids)
    }

    /// Get coordinator statistics
    pub async fn get_stats(&self) -> Result<CoordinatorStats, String> {
        let proposals = self.proposals.read().await;
        let messages = self.messages.read().await;

        let active_proposals = proposals
            .values()
            .filter(|p| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                p.expires_at > now
            })
            .count() as u32;

        Ok(CoordinatorStats {
            total_messages: messages.len() as u32,
            total_proposals: proposals.len() as u32,
            active_proposals,
            consensus_reached: proposals
                .values()
                .filter(|p| p.has_consensus())
                .count() as u32,
        })
    }
}

/// Statistics about the coordinator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorStats {
    pub total_messages: u32,
    pub total_proposals: u32,
    pub active_proposals: u32,
    pub consensus_reached: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        let executor = Arc::new(SwarmExecutor::new());
        let coordinator = AgentCoordinator::new(executor);

        let message = Message::DataRequest {
            from: AgentId("agent-1".to_string()),
            to: AgentId("agent-2".to_string()),
            query: "balance".to_string(),
        };

        coordinator.send_message(message).await.unwrap();
        
        let messages = coordinator
            .get_messages(&AgentId("agent-2".to_string()))
            .await
            .unwrap();
        
        assert_eq!(messages.len(), 1);
    }

    #[tokio::test]
    async fn test_create_proposal() {
        let executor = Arc::new(SwarmExecutor::new());
        let coordinator = AgentCoordinator::new(executor);

        let proposal_id = coordinator
            .create_proposal(
                AgentId("agent-1".to_string()),
                "Test Proposal".to_string(),
                "A test proposal".to_string(),
                vec![
                    AgentId("agent-1".to_string()),
                    AgentId("agent-2".to_string()),
                    AgentId("agent-3".to_string()),
                ],
            )
            .await
            .unwrap();

        let proposal = coordinator.get_proposal(proposal_id).await.unwrap();
        assert!(proposal.is_some());
    }

    #[test]
    fn test_consensus_voting() {
        let mut proposal = Proposal {
            id: 1,
            proposer: AgentId("agent-1".to_string()),
            title: "Test".to_string(),
            description: "Test".to_string(),
            recipients: vec![
                AgentId("agent-1".to_string()),
                AgentId("agent-2".to_string()),
                AgentId("agent-3".to_string()),
            ],
            votes: HashMap::new(),
            created_at: 0,
            expires_at: 3600,
        };

        // Add votes
        proposal.votes.insert(AgentId("agent-1".to_string()), VoteChoice::Yes);
        proposal.votes.insert(AgentId("agent-2".to_string()), VoteChoice::Yes);

        assert!(proposal.has_consensus());
    }
}
