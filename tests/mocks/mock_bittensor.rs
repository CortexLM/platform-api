// Mock Bittensor chain client for testing
// Bittensor requires external network, so we always mock it

use anyhow::Result;
use std::collections::HashMap;

/// Mock Bittensor chain client for testing
pub struct MockBittensorClient {
    pub mock_neurons: HashMap<String, MockNeuron>,
}

#[derive(Clone)]
pub struct MockNeuron {
    pub hotkey: String,
    pub stake: f64,
    pub rank: u32,
}

impl MockBittensorClient {
    pub fn new() -> Self {
        Self {
            mock_neurons: HashMap::new(),
        }
    }

    pub fn with_neuron(mut self, hotkey: String, stake: f64, rank: u32) -> Self {
        self.mock_neurons.insert(hotkey.clone(), MockNeuron {
            hotkey,
            stake,
            rank,
        });
        self
    }

    /// Mock querying neurons from chain
    pub async fn query_neurons(&self, _netuid: u64) -> Result<Vec<MockNeuron>> {
        Ok(self.mock_neurons.values().cloned().collect())
    }

    /// Mock getting neuron by hotkey
    pub async fn get_neuron(&self, hotkey: &str) -> Result<Option<MockNeuron>> {
        Ok(self.mock_neurons.get(hotkey).cloned())
    }
}

impl Default for MockBittensorClient {
    fn default() -> Self {
        Self::new()
    }
}

