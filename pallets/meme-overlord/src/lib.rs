//! # Meme Overlord Pallet
//!
//! Auto-generates celebratory memes for profitable trades on Atlas Sphere.
//!
//! ## Overview
//!
//! The Meme Overlord Pallet automatically tracks profitable trades and generates
//! meme content based on trade performance, creating a fun social layer for DeFi.
//!
//! ## Features
//!
//! - **Trade Achievement System**: Track milestones and achievements
//! - **Meme Template Registry**: On-chain meme template storage
//! - **Auto-generation**: Generate memes based on trade results
//! - **Social Rewards**: Earn MEME tokens for viral trades
//! - **Leaderboard**: Track top traders and their meme collections
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     MEME OVERLORD PALLET                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
//! │  │ Trade Listener  │──│  Achievement    │──│  Meme Generator │ │
//! │  │                 │  │  Tracker        │  │                 │ │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
//! │           │                    │                    │          │
//! │           ▼                    ▼                    ▼          │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                    Storage Layer                        │   │
//! │  │  Templates | Achievements | Memes | Leaderboard        │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! │           │                    │                    │          │
//! │           ▼                    ▼                    ▼          │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │                    Event Emission                       │   │
//! │  │  MemeGenerated | AchievementUnlocked | Viral            │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
    use crate::weights::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::SaturatedConversion;
    use sp_std::prelude::*;

    /// Pallet configuration trait
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Weight information for extrinsics
        type WeightInfo: WeightInfo;

        /// Maximum template name length
        #[pallet::constant]
        type MaxTemplateNameLength: Get<u32>;

        /// Maximum meme data length (IPFS hash or data URL)
        #[pallet::constant]
        type MaxMemeDataLength: Get<u32>;

        /// Maximum achievements per account
        #[pallet::constant]
        type MaxAchievements: Get<u32>;

        /// Minimum profit for meme generation (in basis points)
        #[pallet::constant]
        type MinProfitBps: Get<u32>;

        /// Viral threshold (likes/shares needed)
        #[pallet::constant]
        type ViralThreshold: Get<u32>;
    }

    use frame_support::traits::StorageVersion;

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    // ========== STORAGE ==========

    /// Registered meme templates
    #[pallet::storage]
    #[pallet::getter(fn templates)]
    pub type Templates<T: Config> =
        StorageMap<_, Blake2_128Concat, TemplateId, MemeTemplate<T>, OptionQuery>;

    /// Next template ID
    #[pallet::storage]
    #[pallet::getter(fn next_template_id)]
    pub type NextTemplateId<T: Config> = StorageValue<_, TemplateId, ValueQuery>;

    /// Generated memes
    #[pallet::storage]
    #[pallet::getter(fn memes)]
    pub type Memes<T: Config> = StorageMap<_, Blake2_128Concat, MemeId, Meme<T>, OptionQuery>;

    /// Next meme ID
    #[pallet::storage]
    #[pallet::getter(fn next_meme_id)]
    pub type NextMemeId<T: Config> = StorageValue<_, MemeId, ValueQuery>;

    /// User achievements
    #[pallet::storage]
    #[pallet::getter(fn achievements)]
    pub type Achievements<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BoundedVec<Achievement, T::MaxAchievements>,
        ValueQuery,
    >;

    /// Trade statistics per account
    #[pallet::storage]
    #[pallet::getter(fn trader_stats)]
    pub type TraderStats<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, TradeStats, ValueQuery>;

    /// Leaderboard (top traders by profit)
    #[pallet::storage]
    #[pallet::getter(fn leaderboard)]
    pub type Leaderboard<T: Config> =
        StorageValue<_, BoundedVec<(T::AccountId, u128), ConstU32<100>>, ValueQuery>;

    /// Meme social engagement (likes, shares)
    #[pallet::storage]
    #[pallet::getter(fn meme_engagement)]
    pub type MemeEngagement<T: Config> =
        StorageMap<_, Blake2_128Concat, MemeId, Engagement, ValueQuery>;

    /// Viral memes
    #[pallet::storage]
    #[pallet::getter(fn viral_memes)]
    pub type ViralMemes<T: Config> = StorageValue<_, BoundedVec<MemeId, ConstU32<50>>, ValueQuery>;

    // ========== TYPES ==========

    /// Template ID type
    pub type TemplateId = u64;

    /// Meme ID type
    pub type MemeId = u64;

    /// Meme template structure
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    #[scale_info(skip_type_params(T))]
    pub struct MemeTemplate<T: Config> {
        /// Template ID
        pub id: TemplateId,
        /// Template name
        pub name: BoundedVec<u8, T::MaxTemplateNameLength>,
        /// Template category
        pub category: MemeCategory,
        /// Base image IPFS hash
        pub image_cid: BoundedVec<u8, T::MaxMemeDataLength>,
        /// Text positions for overlay
        pub text_positions: TextPositions,
        /// Creator account
        pub creator: T::AccountId,
        /// Usage count
        pub usage_count: u32,
        /// Is active
        pub is_active: bool,
    }

    /// Generated meme
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    #[scale_info(skip_type_params(T))]
    pub struct Meme<T: Config> {
        /// Meme ID
        pub id: MemeId,
        /// Template used
        pub template_id: TemplateId,
        /// Generated image CID
        pub image_cid: BoundedVec<u8, T::MaxMemeDataLength>,
        /// Trader account
        pub trader: T::AccountId,
        /// Trade that triggered this meme
        pub trade_hash: [u8; 32],
        /// Profit in basis points
        pub profit_bps: u32,
        /// Achievement unlocked
        pub achievement: Option<AchievementType>,
        /// Generated at block
        pub generated_at: BlockNumberFor<T>,
        /// Is viral
        pub is_viral: bool,
    }

    /// Meme categories
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Copy)]
    pub enum MemeCategory {
        /// Small profit memes (1-10%)
        SmallGain,
        /// Medium profit memes (10-50%)
        MediumGain,
        /// Large profit memes (50-100%)
        LargeGain,
        /// Massive profit memes (100%+)
        MassiveGain,
        /// Loss consolation memes
        Loss,
        /// Milestone achievement memes
        Milestone,
        /// Streak memes (consecutive wins)
        Streak,
        /// Diamond hands memes (holding through volatility)
        DiamondHands,
    }

    /// Text positions for meme overlay
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct TextPositions {
        pub top_x: u16,
        pub top_y: u16,
        pub bottom_x: u16,
        pub bottom_y: u16,
    }

    /// Achievement types
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Copy)]
    pub enum AchievementType {
        /// First trade
        FirstTrade,
        /// First profit
        FirstProfit,
        /// 10x profit
        TenBagger,
        /// 100x profit
        HundredBagger,
        /// 1000x profit (legendary)
        ThousandBagger,
        /// 10 consecutive wins
        WinStreak10,
        /// 50 consecutive wins
        WinStreak50,
        /// 100 consecutive wins
        WinStreak100,
        /// First viral meme
        FirstViral,
        /// 10 viral memes
        MemeKing,
        /// Held through 50% drop and recovered
        DiamondHands,
        /// First cross-chain trade
        BridgeBuilder,
        /// Million dollar trade
        Millionaire,
        /// Trade during high volatility
        VolatilityMaster,
        /// Early adopter
        OG,
    }

    /// Achievement record
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
    pub struct Achievement {
        pub achievement_type: AchievementType,
        pub unlocked_at: u64,
        pub trade_hash: Option<[u8; 32]>,
    }

    /// Trade statistics
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct TradeStats {
        pub total_trades: u64,
        pub profitable_trades: u64,
        pub total_profit: u128,
        pub total_loss: u128,
        pub current_streak: u32,
        pub best_streak: u32,
        pub memes_generated: u32,
        pub viral_memes: u32,
    }

    /// Social engagement for a meme
    #[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Default)]
    pub struct Engagement {
        pub likes: u32,
        pub shares: u32,
        pub comments: u32,
    }

    // ========== EVENTS ==========

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new meme template was registered
        TemplateRegistered {
            template_id: TemplateId,
            creator: T::AccountId,
            category: MemeCategory,
        },
        /// A meme was generated for a trade
        MemeGenerated {
            meme_id: MemeId,
            trader: T::AccountId,
            template_id: TemplateId,
            profit_bps: u32,
        },
        /// An achievement was unlocked
        AchievementUnlocked {
            trader: T::AccountId,
            achievement: AchievementType,
        },
        /// A meme went viral
        MemeWentViral {
            meme_id: MemeId,
            trader: T::AccountId,
            likes: u32,
            shares: u32,
        },
        /// Leaderboard was updated
        LeaderboardUpdated {
            trader: T::AccountId,
            new_rank: u32,
            total_profit: u128,
        },
        /// Meme was liked
        MemeLiked {
            meme_id: MemeId,
            liker: T::AccountId,
        },
        /// Meme was shared
        MemeShared {
            meme_id: MemeId,
            sharer: T::AccountId,
        },
    }

    // ========== ERRORS ==========

    #[pallet::error]
    pub enum Error<T> {
        /// Template not found
        TemplateNotFound,
        /// Template name too long
        TemplateNameTooLong,
        /// Meme data too long
        MemeDataTooLong,
        /// Meme not found
        MemeNotFound,
        /// Maximum achievements reached
        MaxAchievementsReached,
        /// Already liked this meme
        AlreadyLiked,
        /// Profit too low for meme generation
        ProfitTooLow,
        /// Template is inactive
        TemplateInactive,
        /// Not authorized
        NotAuthorized,
    }

    // ========== EXTRINSICS ==========

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new meme template
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_template())]
        pub fn register_template(
            origin: OriginFor<T>,
            name: Vec<u8>,
            category: MemeCategory,
            image_cid: Vec<u8>,
            text_positions: TextPositions,
        ) -> DispatchResult {
            let creator = ensure_signed(origin)?;

            let bounded_name: BoundedVec<u8, T::MaxTemplateNameLength> = name
                .try_into()
                .map_err(|_| Error::<T>::TemplateNameTooLong)?;

            let bounded_cid: BoundedVec<u8, T::MaxMemeDataLength> = image_cid
                .try_into()
                .map_err(|_| Error::<T>::MemeDataTooLong)?;

            let template_id = NextTemplateId::<T>::get();
            NextTemplateId::<T>::put(template_id + 1);

            let template = MemeTemplate {
                id: template_id,
                name: bounded_name,
                category,
                image_cid: bounded_cid,
                text_positions,
                creator: creator.clone(),
                usage_count: 0,
                is_active: true,
            };

            Templates::<T>::insert(template_id, template);

            Self::deposit_event(Event::TemplateRegistered {
                template_id,
                creator,
                category,
            });

            Ok(())
        }

        /// Generate meme for a trade (called by atlas-kernel after profitable trade)
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::generate_meme())]
        pub fn generate_meme(
            origin: OriginFor<T>,
            trade_hash: [u8; 32],
            profit_bps: u32,
        ) -> DispatchResult {
            let trader = ensure_signed(origin)?;

            // Check minimum profit
            ensure!(
                profit_bps >= T::MinProfitBps::get(),
                Error::<T>::ProfitTooLow
            );

            // Select template based on profit
            let category = Self::category_for_profit(profit_bps);
            let template_id = Self::select_template(category)?;

            // Generate meme
            let meme_id = Self::do_generate_meme(&trader, template_id, trade_hash, profit_bps)?;

            // Update stats
            Self::update_trader_stats(&trader, profit_bps);

            // Check achievements
            Self::check_achievements(&trader, profit_bps)?;

            Self::deposit_event(Event::MemeGenerated {
                meme_id,
                trader,
                template_id,
                profit_bps,
            });

            Ok(())
        }

        /// Like a meme
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::like_meme())]
        pub fn like_meme(origin: OriginFor<T>, meme_id: MemeId) -> DispatchResult {
            let liker = ensure_signed(origin)?;

            ensure!(Memes::<T>::contains_key(meme_id), Error::<T>::MemeNotFound);

            MemeEngagement::<T>::mutate(meme_id, |engagement| {
                engagement.likes += 1;
            });

            // Check if went viral
            Self::check_viral_status(meme_id)?;

            Self::deposit_event(Event::MemeLiked { meme_id, liker });

            Ok(())
        }

        /// Share a meme
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::share_meme())]
        pub fn share_meme(origin: OriginFor<T>, meme_id: MemeId) -> DispatchResult {
            let sharer = ensure_signed(origin)?;

            ensure!(Memes::<T>::contains_key(meme_id), Error::<T>::MemeNotFound);

            MemeEngagement::<T>::mutate(meme_id, |engagement| {
                engagement.shares += 1;
            });

            // Check if went viral
            Self::check_viral_status(meme_id)?;

            Self::deposit_event(Event::MemeShared { meme_id, sharer });

            Ok(())
        }

        /// Deactivate a template (only creator)
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::deactivate_template())]
        pub fn deactivate_template(
            origin: OriginFor<T>,
            template_id: TemplateId,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            Templates::<T>::try_mutate(template_id, |maybe_template| -> DispatchResult {
                let template = maybe_template
                    .as_mut()
                    .ok_or(Error::<T>::TemplateNotFound)?;
                ensure!(template.creator == who, Error::<T>::NotAuthorized);
                template.is_active = false;
                Ok(())
            })?;

            Ok(())
        }
    }

    // ========== IMPLEMENTATION ==========

    impl<T: Config> Pallet<T> {
        /// Get category for profit level
        fn category_for_profit(profit_bps: u32) -> MemeCategory {
            match profit_bps {
                0..=999 => MemeCategory::SmallGain,      // 0-10%
                1000..=4999 => MemeCategory::MediumGain, // 10-50%
                5000..=9999 => MemeCategory::LargeGain,  // 50-100%
                _ => MemeCategory::MassiveGain,          // 100%+
            }
        }

        /// Select a template for the given category
        fn select_template(category: MemeCategory) -> Result<TemplateId, Error<T>> {
            // Find an active template matching the category
            // In production, would use randomness for variety
            for (id, template) in Templates::<T>::iter() {
                if template.category == category && template.is_active {
                    return Ok(id);
                }
            }

            // Fallback to any active template
            for (id, template) in Templates::<T>::iter() {
                if template.is_active {
                    return Ok(id);
                }
            }

            Err(Error::<T>::TemplateNotFound)
        }

        /// Generate a meme
        fn do_generate_meme(
            trader: &T::AccountId,
            template_id: TemplateId,
            trade_hash: [u8; 32],
            profit_bps: u32,
        ) -> Result<MemeId, DispatchError> {
            let template = Templates::<T>::get(template_id).ok_or(Error::<T>::TemplateNotFound)?;

            ensure!(template.is_active, Error::<T>::TemplateInactive);

            let meme_id = NextMemeId::<T>::get();
            NextMemeId::<T>::put(meme_id + 1);

            // Generate meme image CID (would be done off-chain in production)
            let image_cid = template.image_cid.clone();

            let meme = Meme {
                id: meme_id,
                template_id,
                image_cid,
                trader: trader.clone(),
                trade_hash,
                profit_bps,
                achievement: None,
                generated_at: frame_system::Pallet::<T>::block_number(),
                is_viral: false,
            };

            Memes::<T>::insert(meme_id, meme);

            // Update template usage count
            Templates::<T>::mutate(template_id, |maybe_template| {
                if let Some(template) = maybe_template {
                    template.usage_count += 1;
                }
            });

            Ok(meme_id)
        }

        /// Update trader statistics
        fn update_trader_stats(trader: &T::AccountId, profit_bps: u32) {
            TraderStats::<T>::mutate(trader, |stats| {
                stats.total_trades += 1;
                if profit_bps > 0 {
                    stats.profitable_trades += 1;
                    stats.total_profit += profit_bps as u128;
                    stats.current_streak += 1;
                    if stats.current_streak > stats.best_streak {
                        stats.best_streak = stats.current_streak;
                    }
                } else {
                    stats.current_streak = 0;
                }
                stats.memes_generated += 1;
            });

            // Update leaderboard
            Self::update_leaderboard(trader);
        }

        /// Update the leaderboard
        fn update_leaderboard(trader: &T::AccountId) {
            let stats = TraderStats::<T>::get(trader);

            Leaderboard::<T>::mutate(|leaderboard| {
                // Remove existing entry for this trader
                leaderboard.retain(|(account, _)| account != trader);

                // Add new entry
                let _ = leaderboard.try_push((trader.clone(), stats.total_profit));

                // Sort by profit (descending)
                leaderboard.sort_by(|a, b| b.1.cmp(&a.1));

                // Truncate to top 100
                leaderboard.truncate(100);
            });
        }

        /// Check and unlock achievements
        fn check_achievements(trader: &T::AccountId, profit_bps: u32) -> DispatchResult {
            let stats = TraderStats::<T>::get(trader);
            let mut achievements_to_add = Vec::new();

            // First trade
            if stats.total_trades == 1 {
                achievements_to_add.push(AchievementType::FirstTrade);
            }

            // First profit
            if stats.profitable_trades == 1 {
                achievements_to_add.push(AchievementType::FirstProfit);
            }

            // 10x profit (1000% = 100000 bps)
            if profit_bps >= 100000 {
                achievements_to_add.push(AchievementType::TenBagger);
            }

            // 100x profit
            if profit_bps >= 1000000 {
                achievements_to_add.push(AchievementType::HundredBagger);
            }

            // Win streaks
            if stats.current_streak >= 10 {
                achievements_to_add.push(AchievementType::WinStreak10);
            }
            if stats.current_streak >= 50 {
                achievements_to_add.push(AchievementType::WinStreak50);
            }
            if stats.current_streak >= 100 {
                achievements_to_add.push(AchievementType::WinStreak100);
            }

            // Add achievements
            for achievement_type in achievements_to_add {
                if !Self::has_achievement(trader, achievement_type) {
                    Self::add_achievement(trader, achievement_type)?;
                }
            }

            Ok(())
        }

        /// Check if trader has achievement
        fn has_achievement(trader: &T::AccountId, achievement_type: AchievementType) -> bool {
            Achievements::<T>::get(trader)
                .iter()
                .any(|a| a.achievement_type == achievement_type)
        }

        /// Add achievement to trader
        fn add_achievement(
            trader: &T::AccountId,
            achievement_type: AchievementType,
        ) -> DispatchResult {
            let achievement = Achievement {
                achievement_type,
                unlocked_at: frame_system::Pallet::<T>::block_number().saturated_into(),
                trade_hash: None,
            };

            Achievements::<T>::try_mutate(trader, |achievements| {
                achievements
                    .try_push(achievement)
                    .map_err(|_| Error::<T>::MaxAchievementsReached)
            })?;

            Self::deposit_event(Event::AchievementUnlocked {
                trader: trader.clone(),
                achievement: achievement_type,
            });

            Ok(())
        }

        /// Check if meme went viral
        fn check_viral_status(meme_id: MemeId) -> DispatchResult {
            let engagement = MemeEngagement::<T>::get(meme_id);
            let threshold = T::ViralThreshold::get();

            if engagement.likes + engagement.shares >= threshold {
                Memes::<T>::try_mutate(meme_id, |maybe_meme| -> DispatchResult {
                    if let Some(meme) = maybe_meme {
                        if !meme.is_viral {
                            meme.is_viral = true;

                            // Add to viral memes
                            ViralMemes::<T>::mutate(|viral| {
                                let _ = viral.try_push(meme_id);
                            });

                            // Update trader stats
                            TraderStats::<T>::mutate(&meme.trader, |stats| {
                                stats.viral_memes += 1;
                            });

                            // Check viral achievement
                            let trader = meme.trader.clone();
                            let stats = TraderStats::<T>::get(&trader);
                            if stats.viral_memes == 1 {
                                let _ = Self::add_achievement(&trader, AchievementType::FirstViral);
                            }
                            if stats.viral_memes >= 10 {
                                let _ = Self::add_achievement(&trader, AchievementType::MemeKing);
                            }

                            Self::deposit_event(Event::MemeWentViral {
                                meme_id,
                                trader: meme.trader.clone(),
                                likes: engagement.likes,
                                shares: engagement.shares,
                            });
                        }
                    }
                    Ok(())
                })?;
            }

            Ok(())
        }
    }
}
