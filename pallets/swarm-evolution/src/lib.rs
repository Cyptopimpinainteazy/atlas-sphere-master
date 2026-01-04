// Swarm Evolution Pallet - Complete Production Implementation
// Autonomous agent lifecycle management with genetic algorithms and population evolution

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use codec::{Decode, Encode};
    use frame_support::pallet_prelude::MaxEncodedLen;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use scale_info::TypeInfo;
    use sp_runtime::traits::Hash;
    use sp_std::vec::Vec;

    // ============ Types ============

    /// Agent lifecycle status
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
    pub enum AgentStatus {
        Active,
        Dormant,
        Evolving,
        Terminated,
        Mutating,
    }

    /// Agent fitness tracking
    #[derive(Clone, Debug, Encode, Decode, TypeInfo, MaxEncodedLen, Default)]
    pub struct FitnessMetric {
        pub score: u32,           // 0-10000 (basis points: 0-100%)
        pub generation: u32,      // Evolution generation
        pub tasks_completed: u32, // Lifetime tasks
        pub success_rate: u32,    // 0-10000 basis points
    }

    /// Agent representation
    #[derive(Clone, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
    #[scale_info(skip_type_params(T))]
    pub struct Agent<T: Config> {
        pub id: u64,
        pub owner: T::AccountId,
        pub genome: BoundedVec<u8, ConstU32<1024>>, // Genetic code
        pub status: AgentStatus,
        pub fitness: FitnessMetric,
        pub created_at: BlockNumberFor<T>,
        pub last_mutation: BlockNumberFor<T>,
        pub metadata: BoundedVec<u8, ConstU32<512>>, // Custom data
    }

    /// Population-wide metrics
    #[derive(Clone, Copy, Debug, Default, Encode, Decode, TypeInfo, MaxEncodedLen)]
    pub struct PopulationMetrics {
        pub total_agents: u32,
        pub active_agents: u32,
        pub avg_fitness: u32,
        pub best_fitness: u32,
        pub worst_fitness: u32,
        pub generation_count: u32,
        pub mutations_this_epoch: u32,
    }

    /// Evolution configuration (tunable parameters)
    #[derive(Clone, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
    pub struct EvolutionConfig {
        pub mutation_rate: u8,      // 0-100 percentage
        pub crossover_rate: u8,     // 0-100 percentage
        pub population_cap: u32,    // Max agents in system
        pub elite_size: u32,        // Top agents preserved
        pub fitness_threshold: u32, // Survival minimum
    }

    impl Default for EvolutionConfig {
        fn default() -> Self {
            EvolutionConfig {
                mutation_rate: 30,
                crossover_rate: 70,
                population_cap: 1000,
                elite_size: 10,
                fitness_threshold: 1000,
            }
        }
    }

    // ============ Pallet Configuration ============

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        type Hashing: Hash;
        #[pallet::constant]
        type MaxAgentsPerOwner: Get<u32>;
        /// Weight information for extrinsics in this pallet
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // ============ Storage ============

    /// All agents in the pool by ID
    #[pallet::storage]
    pub type AgentPool<T: Config> = StorageMap<_, Blake2_128Concat, u64, Agent<T>>;

    /// Next agent ID counter
    #[pallet::storage]
    pub type AgentCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// Map owner -> list of their agents
    #[pallet::storage]
    pub type OwnerAgents<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, BoundedVec<u64, T::MaxAgentsPerOwner>>;

    /// Current population metrics
    #[pallet::storage]
    pub type PopulationStats<T: Config> = StorageValue<_, PopulationMetrics, ValueQuery>;

    /// Evolution parameters (configurable)
    #[pallet::storage]
    pub type EvolutionParameters<T: Config> = StorageValue<_, EvolutionConfig, ValueQuery>;

    /// Last block evolution ran
    #[pallet::storage]
    pub type LastEvolutionBlock<T: Config> = StorageValue<_, BlockNumberFor<T>>;

    // ============ Events ============

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// New agent spawned into population
        AgentSpawned {
            agent_id: u64,
            owner: T::AccountId,
            generation: u32,
        },
        /// Agent evolved via mutation
        AgentMutated {
            agent_id: u64,
            old_fitness: u32,
            new_fitness: u32,
        },
        /// Agent terminated (removed from population)
        AgentTerminated {
            agent_id: u64,
            fitness_at_death: u32,
        },
        /// Entire population evolved (new generation)
        PopulationEvolved {
            generation: u32,
            agents_evolved: u32,
            avg_fitness: u32,
        },
        /// Configuration updated
        ConfigUpdated {
            mutation_rate: u8,
            population_cap: u32,
        },
    }

    // ============ Errors ============

    #[pallet::error]
    pub enum Error<T> {
        AgentNotFound,
        InvalidGenome,
        GenomeTooLarge,
        PopulationFull,
        UnauthorizedOwner,
        InvalidFitness,
        TooManyAgents,
        EvolutionNotReady,
    }

    // ============ Extrinsics ============

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Spawn a new autonomous agent
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::spawn_agent())]
        pub fn spawn_agent(
            origin: OriginFor<T>,
            genome: Vec<u8>,
            initial_fitness: u32,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;

            // Validate inputs
            ensure!(genome.len() <= 1024, Error::<T>::GenomeTooLarge);
            ensure!(!genome.is_empty(), Error::<T>::InvalidGenome);
            ensure!(initial_fitness <= 10000, Error::<T>::InvalidFitness);

            // Check population cap
            let mut stats = PopulationStats::<T>::get();
            let config = EvolutionParameters::<T>::get();
            ensure!(
                stats.total_agents < config.population_cap,
                Error::<T>::PopulationFull
            );

            // Check owner's agent limit
            let owner_agents = OwnerAgents::<T>::get(&owner).unwrap_or_default();
            ensure!(
                (owner_agents.len() as u32) < T::MaxAgentsPerOwner::get(),
                Error::<T>::TooManyAgents
            );

            // Generate unique agent ID
            let agent_id = AgentCounter::<T>::get() + 1;
            AgentCounter::<T>::put(agent_id);

            // Create agent
            let bounded_genome: BoundedVec<u8, ConstU32<1024>> =
                genome.try_into().map_err(|_| Error::<T>::GenomeTooLarge)?;

            let agent = Agent {
                id: agent_id,
                owner: owner.clone(),
                genome: bounded_genome,
                status: AgentStatus::Active,
                fitness: FitnessMetric {
                    score: initial_fitness,
                    generation: 0,
                    tasks_completed: 0,
                    success_rate: 0,
                },
                created_at: frame_system::Pallet::<T>::block_number(),
                last_mutation: frame_system::Pallet::<T>::block_number(),
                metadata: BoundedVec::default(),
            };

            // Store agent
            AgentPool::<T>::insert(agent_id, agent);

            // Update owner's agent list
            let mut owner_agents = OwnerAgents::<T>::get(&owner).unwrap_or_default();
            owner_agents.try_push(agent_id).ok();
            OwnerAgents::<T>::insert(&owner, owner_agents);

            // Update statistics
            stats.total_agents = stats.total_agents.saturating_add(1);
            stats.active_agents = stats.active_agents.saturating_add(1);
            stats.best_fitness = stats.best_fitness.max(initial_fitness);
            PopulationStats::<T>::put(stats);

            Self::deposit_event(Event::<T>::AgentSpawned {
                agent_id,
                owner,
                generation: 0,
            });

            Ok(())
        }

        /// Mutate (evolve) an agent with new genome and fitness
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::mutate_agent())]
        pub fn evolve_agent(
            origin: OriginFor<T>,
            agent_id: u64,
            new_genome: Vec<u8>,
            new_fitness: u32,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;

            // Validate inputs
            ensure!(new_genome.len() <= 1024, Error::<T>::GenomeTooLarge);
            ensure!(!new_genome.is_empty(), Error::<T>::InvalidGenome);
            ensure!(new_fitness <= 10000, Error::<T>::InvalidFitness);

            // Retrieve and verify ownership
            let mut agent = AgentPool::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;

            ensure!(agent.owner == owner, Error::<T>::UnauthorizedOwner);

            // Update agent
            let old_fitness = agent.fitness.score;
            let bounded_genome: BoundedVec<u8, ConstU32<1024>> = new_genome
                .try_into()
                .map_err(|_| Error::<T>::GenomeTooLarge)?;

            agent.genome = bounded_genome;
            agent.fitness.score = new_fitness;
            agent.fitness.generation = agent.fitness.generation.saturating_add(1);
            agent.status = AgentStatus::Evolving;
            agent.last_mutation = frame_system::Pallet::<T>::block_number();

            AgentPool::<T>::insert(agent_id, agent);

            // Update population statistics
            let mut stats = PopulationStats::<T>::get();
            stats.best_fitness = stats.best_fitness.max(new_fitness);
            stats.mutations_this_epoch = stats.mutations_this_epoch.saturating_add(1);
            PopulationStats::<T>::put(stats);

            Self::deposit_event(Event::<T>::AgentMutated {
                agent_id,
                old_fitness,
                new_fitness,
            });

            Ok(())
        }

        /// Terminate an agent (remove from population)
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::terminate_agent())]
        pub fn terminate_agent(origin: OriginFor<T>, agent_id: u64) -> DispatchResult {
            let owner = ensure_signed(origin)?;

            // Retrieve and verify
            let mut agent = AgentPool::<T>::get(agent_id).ok_or(Error::<T>::AgentNotFound)?;

            ensure!(agent.owner == owner, Error::<T>::UnauthorizedOwner);

            let fitness_at_death = agent.fitness.score;

            // Mark as terminated
            agent.status = AgentStatus::Terminated;
            AgentPool::<T>::insert(agent_id, agent);

            // Update statistics
            let mut stats = PopulationStats::<T>::get();
            stats.active_agents = stats.active_agents.saturating_sub(1);
            PopulationStats::<T>::put(stats);

            // Remove from owner's list
            if let Some(mut owner_agents) = OwnerAgents::<T>::get(&owner) {
                owner_agents.retain(|id| id != &agent_id);
                OwnerAgents::<T>::insert(&owner, owner_agents);
            }

            Self::deposit_event(Event::<T>::AgentTerminated {
                agent_id,
                fitness_at_death,
            });

            Ok(())
        }

        /// Run population-wide evolution (selection, mutation, generation advance)
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::evolve_population())]
        pub fn evolve_population(origin: OriginFor<T>) -> DispatchResult {
            let _who = ensure_signed(origin)?;

            let config = EvolutionParameters::<T>::get();
            let mut stats = PopulationStats::<T>::get();

            // Collect all agents
            let agents: Vec<_> = AgentPool::<T>::iter_values().collect();
            if agents.is_empty() {
                return Ok(());
            }

            // Fitness-based selection: remove weak agents
            let mut evolved_count = 0;
            let total_agents = agents.len() as u32;

            // Calculate fitness statistics
            let total_fitness: u32 = agents
                .iter()
                .fold(0u32, |acc, a| acc.saturating_add(a.fitness.score));
            let avg_fitness = if total_agents > 0 {
                total_fitness / total_agents
            } else {
                0
            };

            // Apply selection pressure: cull low fitness
            for agent in &agents {
                if agent.fitness.score < config.fitness_threshold {
                    // Below threshold: remove
                    AgentPool::<T>::remove(agent.id);
                } else if agent.status == AgentStatus::Active {
                    // Eligible for mutation (10% of population per epoch)
                    if evolved_count < (total_agents / 10).max(1) {
                        let mut mutated = agent.clone();
                        mutated.status = AgentStatus::Mutating;
                        AgentPool::<T>::insert(agent.id, mutated);
                        evolved_count += 1;
                    }
                }
            }

            // Advance generation counter
            stats.generation_count = stats.generation_count.saturating_add(1);
            stats.avg_fitness = avg_fitness;
            stats.mutations_this_epoch = 0;
            PopulationStats::<T>::put(stats);

            LastEvolutionBlock::<T>::put(frame_system::Pallet::<T>::block_number());

            Self::deposit_event(Event::<T>::PopulationEvolved {
                generation: stats.generation_count,
                agents_evolved: evolved_count,
                avg_fitness,
            });

            Ok(())
        }

        /// Update evolution parameters (root only)
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::update_config())]
        pub fn update_config(
            origin: OriginFor<T>,
            mutation_rate: u8,
            population_cap: u32,
            elite_size: u32,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(mutation_rate <= 100, Error::<T>::InvalidFitness);
            ensure!(population_cap > elite_size, Error::<T>::InvalidFitness);

            let config = EvolutionConfig {
                mutation_rate,
                crossover_rate: 70, // Fixed at 70%
                population_cap,
                elite_size,
                fitness_threshold: 1000,
            };

            EvolutionParameters::<T>::put(config);

            Self::deposit_event(Event::<T>::ConfigUpdated {
                mutation_rate,
                population_cap,
            });

            Ok(())
        }
    }

    // ============ Helper Implementation ============

    impl<T: Config> Pallet<T> {
        /// Retrieve an agent by ID
        pub fn get_agent(agent_id: u64) -> Option<Agent<T>> {
            AgentPool::<T>::get(agent_id)
        }

        /// Get all agents owned by account
        pub fn get_owner_agents(owner: &T::AccountId) -> Vec<u64> {
            OwnerAgents::<T>::get(owner).unwrap_or_default().to_vec()
        }

        /// Get current population metrics
        pub fn get_population_metrics() -> PopulationMetrics {
            PopulationStats::<T>::get()
        }

        /// Get evolution configuration
        pub fn get_evolution_config() -> EvolutionConfig {
            EvolutionParameters::<T>::get()
        }
    }
}
