//! Lock-Free Ring Buffer IPC & Swarm Consensus Aggregator.
//!
//! Provides bounded MPMC ring channels, deterministic execution level synchronization,
//! fault-tolerant agent reassignment, and verifiable consensus receipts for 300-agent swarms.

#![forbid(unsafe_code)]

pub mod consensus_aggregator;
pub mod ring_channel;
pub mod types;

pub use consensus_aggregator::{SwarmConsensusAggregator, TaskCompletionRecord};
pub use ring_channel::{SwarmRingChannel, DEFAULT_RING_CAPACITY};
pub use types::{
    ChannelError, ChannelStats, ConsensusStatus, MessageId, SwarmConsensusReceipt,
    SwarmMessage, SwarmMessageType,
};

// ---------------------------------------------------------------------------
// Unit & Integration Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::socratic::contract::types::SpecContract;
    use crate::intent::socratic::dialogue::types::ArchitecturalPattern;
    use crate::swarm::dag::planner::SwarmDagPlanner;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn test_multithreaded_ring_channel_10k_messages() {
        let channel = SwarmRingChannel::new(20000);
        let total_messages = 10000;
        let num_producers = 4;
        let msgs_per_producer = total_messages / num_producers;

        let received_count = Arc::new(AtomicUsize::new(0));

        // Spawn consumer thread
        let consumer_channel = channel.clone();
        let consumer_received = received_count.clone();
        let consumer_handle = thread::spawn(move || {
            let mut count = 0;
            while count < total_messages {
                if let Ok(_msg) = consumer_channel.try_recv() {
                    count += 1;
                    consumer_received.fetch_add(1, Ordering::Relaxed);
                } else {
                    thread::yield_now();
                }
            }
        });

        // Spawn producer threads
        let mut producer_handles = Vec::new();
        for prod_id in 0..num_producers {
            let prod_channel = channel.clone();
            let handle = thread::spawn(move || {
                for seq in 0..msgs_per_producer {
                    let msg = SwarmMessage::new(
                        prod_id as u32,
                        0,
                        seq as u64,
                        SwarmMessageType::TaskProgress,
                        vec![1, 2, 3, 4],
                    );
                    while prod_channel.try_send(msg.clone()).is_err() {
                        thread::yield_now();
                    }
                }
            });
            producer_handles.push(handle);
        }

        for h in producer_handles {
            h.join().unwrap();
        }
        consumer_handle.join().unwrap();

        assert_eq!(received_count.load(Ordering::Relaxed), total_messages);
        let stats = channel.stats();
        assert_eq!(stats.messages_sent, total_messages as u64);
        assert_eq!(stats.messages_received, total_messages as u64);
        assert_eq!(stats.dropped_messages, 0);
    }

    #[test]
    fn test_50_task_parallel_dag_consensus_and_quorum() {
        let contract = SpecContract::new_draft(
            "Fullstack E-Commerce Engine",
            vec!["OrderService".to_string(), "PaymentGateway".to_string()],
            vec!["src/services/".to_string()],
            ArchitecturalPattern::CleanArchitecture,
        );

        let dag = SwarmDagPlanner::plan_from_contract(&contract, "ECommerceApp")
            .expect("DAG planning succeeds");

        let aggregator = SwarmConsensusAggregator::new(dag.clone());

        // Process level by level
        for (lvl_idx, level_tasks) in dag.execution_levels.iter().enumerate() {
            assert!(
                !aggregator.check_level_quorum(lvl_idx).unwrap(),
                "Level {} should not be at quorum initially",
                lvl_idx
            );

            for (idx, task_id) in level_tasks.iter().enumerate() {
                let agent_id = (idx % 300) as u32;
                let output_hash = 0xabcdef0000 + (task_id.0 % 1000);
                aggregator
                    .complete_task(agent_id, task_id.0, output_hash)
                    .expect("Task completion succeeds");
            }

            assert!(
                aggregator.check_level_quorum(lvl_idx).unwrap(),
                "Level {} should reach quorum after all tasks complete",
                lvl_idx
            );

            let next_lvl = aggregator.advance_level().expect("Level advancement succeeds");
            assert_eq!(next_lvl, lvl_idx + 1);
        }

        assert!(aggregator.is_dag_completed());

        // Finalize consensus and seal receipt
        let receipt = aggregator
            .finalize_consensus()
            .expect("Final consensus receipt generation succeeds");

        assert_eq!(receipt.dag_id, dag.dag_id);
        assert_eq!(receipt.committed_tasks.len(), dag.nodes.len());
        assert_eq!(receipt.status, ConsensusStatus::ConsensusSealed);
        assert_ne!(receipt.final_cst_root_hash, 0);
    }

    #[test]
    fn test_fault_tolerant_task_reassignment() {
        let contract = SpecContract::new_draft(
            "Fault Tolerant Swarm Test",
            vec!["CoreWorker".to_string()],
            vec!["src/core/".to_string()],
            ArchitecturalPattern::MicroKernel,
        );

        let dag = SwarmDagPlanner::plan_from_contract(&contract, "FaultApp")
            .expect("DAG planning succeeds");

        let aggregator = SwarmConsensusAggregator::new(dag.clone());
        let target_task_id = dag.execution_levels[0][0].0;
        let failed_agent_id = 42;
        let healthy_agent_id = 84;

        aggregator
            .assign_task(target_task_id, failed_agent_id)
            .expect("Assignment succeeds");

        // Agent fails mid-execution
        aggregator
            .fail_and_reassign_task(failed_agent_id, target_task_id, healthy_agent_id)
            .expect("Reassignment succeeds");

        // Healthy agent completes the task
        aggregator
            .complete_task(healthy_agent_id, target_task_id, 0x1234567890abcdef)
            .expect("Completion succeeds");

        assert!(aggregator.check_level_quorum(0).unwrap());
    }

    #[test]
    fn test_sub_0_05ms_consensus_aggregation_latency() {
        let contract = SpecContract::new_draft(
            "Latency Benchmark App",
            vec!["FastProcessor".to_string()],
            vec!["src/processor/".to_string()],
            ArchitecturalPattern::DataOriented,
        );

        let dag = SwarmDagPlanner::plan_from_contract(&contract, "FastApp")
            .expect("DAG planning succeeds");

        let iterations = 1000;
        let start = Instant::now();

        for _ in 0..iterations {
            let aggregator = SwarmConsensusAggregator::new(dag.clone());
            for level in &dag.execution_levels {
                for task_id in level {
                    let _ = aggregator.complete_task(1, task_id.0, 0x55555555);
                }
                let _ = aggregator.advance_level();
            }
            let _ = aggregator.finalize_consensus();
        }

        let total_ms = start.elapsed().as_secs_f64() * 1000.0;
        let avg_latency_ms = total_ms / (iterations as f64);

        println!("⚡ Average Swarm Consensus Aggregation Latency: {:.4}ms", avg_latency_ms);

        let budget = if cfg!(debug_assertions) { 0.50 } else { 0.05 };
        assert!(
            avg_latency_ms < budget,
            "Consensus aggregation must satisfy budget ({:.2}ms), got {:.4}ms",
            budget,
            avg_latency_ms
        );
    }
}
