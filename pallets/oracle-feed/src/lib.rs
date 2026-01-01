#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]


#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_std::vec::Vec;

    pub type FeedId = u32;
    pub type Price = u128;

    /// Aggregation method for price feeds
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub enum AggregationMethod {
        Median,
        VolumeWeighted,
        TimeWeighted,
        MedianOfMeans,
    }

    /// Price data point
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    #[scale_info(skip_type_params(T))]
    pub struct PriceData<T: Config> {
        pub feed_id: FeedId,
        pub price: Price,
        pub timestamp: BlockNumberFor<T>,
        pub source: Vec<u8>,  // Oracle name
    }

    /// Price feed configuration
    #[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
    pub struct PriceFeed {
        pub feed_id: FeedId,
        pub name: Vec<u8>,
        pub base_asset: Vec<u8>,
        pub quote_asset: Vec<u8>,
        pub enabled: bool,
        pub confidence_interval: u32,  // bps
        pub heartbeat_blocks: u32,
        pub last_updated: u32,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Maximum number of price sources for aggregation
        #[pallet::constant]
        type MaxPriceSources: Get<u32>;

        /// Minimum sources required for aggregation
        #[pallet::constant]
        type MinPriceSources: Get<u32>;

        /// Maximum acceptable price deviation (bps)
        #[pallet::constant]
        type MaxPriceDeviation: Get<u32>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// All price feeds
    #[pallet::storage]
    #[pallet::getter(fn price_feed)]
    pub type PriceFeeds<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        FeedId,
        PriceFeed,
        OptionQuery,
    >;

    /// Latest prices for each feed
    #[pallet::storage]
    #[pallet::getter(fn latest_price)]
    pub type LatestPrices<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        FeedId,
        (Price, BlockNumberFor<T>),
        OptionQuery,
    >;

    /// Price history (circular buffer)
    #[pallet::storage]
    #[pallet::getter(fn price_history)]
    pub type PriceHistory<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        FeedId,
        Vec<(Price, BlockNumberFor<T>)>,
        ValueQuery,
    >;

    /// Trusted oracle providers
    #[pallet::storage]
    #[pallet::getter(fn trusted_oracle)]
    pub type TrustedOracles<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,  // Oracle name
        bool,
        ValueQuery,
    >;

    /// Feed counter
    #[pallet::storage]
    #[pallet::getter(fn feed_counter)]
    pub type FeedCounter<T: Config> = StorageValue<_, FeedId, ValueQuery>;

    /// Total price submissions
    #[pallet::storage]
    #[pallet::getter(fn total_submissions)]
    pub type TotalSubmissions<T: Config> = StorageValue<_, u32, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Price feed created
        PriceFeedCreated {
            feed_id: FeedId,
            name: Vec<u8>,
            base_asset: Vec<u8>,
        },
        /// Price submitted for feed
        PriceSubmitted {
            feed_id: FeedId,
            price: Price,
            source: Vec<u8>,
        },
        /// Price aggregated from multiple sources
        PriceAggregated {
            feed_id: FeedId,
            aggregated_price: Price,
            source_count: u32,
        },
        /// Oracle provider added to trusted list
        OracleProviderAdded {
            name: Vec<u8>,
        },
        /// Price deviation detected
        PriceDeviationDetected {
            feed_id: FeedId,
            old_price: Price,
            new_price: Price,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Feed not found
        FeedNotFound,
        /// Feed already exists
        FeedExists,
        /// Invalid price (zero or too large)
        InvalidPrice,
        /// Unauthorized oracle provider
        UnauthorizedProvider,
        /// Insufficient price sources for aggregation
        InsufficientSources,
        /// Price deviation too large
        PriceDeviation,
        /// Feed is disabled
        FeedDisabled,
        /// Stale price data
        StalePrice,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
            // Initialize trusted oracles if not set
            Weight::zero()
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new price feed
        #[pallet::call_index(0)]
        #[pallet::weight(5_000)]
        pub fn create_price_feed(
            origin: OriginFor<T>,
            name: Vec<u8>,
            base_asset: Vec<u8>,
            quote_asset: Vec<u8>,
            heartbeat_blocks: u32,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;

            let feed_id = Self::next_feed_id();

            let feed = PriceFeed {
                feed_id,
                name: name.clone(),
                base_asset: base_asset.clone(),
                quote_asset,
                enabled: true,
                confidence_interval: 100,  // 1% default
                heartbeat_blocks,
                last_updated: 0,
            };

            PriceFeeds::<T>::insert(feed_id, feed);

            Self::deposit_event(Event::PriceFeedCreated {
                feed_id,
                name,
                base_asset,
            });

            Ok(())
        }

        /// Submit a price from oracle provider
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn submit_price(
            origin: OriginFor<T>,
            feed_id: FeedId,
            price: Price,
            source: Vec<u8>,
        ) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            // Verify feed exists
            let mut feed = Self::price_feed(feed_id).ok_or(Error::<T>::FeedNotFound)?;
            ensure!(feed.enabled, Error::<T>::FeedDisabled);

            // Verify oracle is trusted
            let is_trusted = TrustedOracles::<T>::get(&source);
            ensure!(is_trusted, Error::<T>::UnauthorizedProvider);

            // Validate price
            ensure!(price > 0, Error::<T>::InvalidPrice);
            ensure!(price <= 1_000_000_000_000_000_000_000_000u128, Error::<T>::InvalidPrice);

            // Check for price deviation
            if let Some((old_price, _)) = Self::latest_price(feed_id) {
                let max_deviation = (old_price * T::MaxPriceDeviation::get() as u128) / 10_000u128;
                let deviation = if price > old_price { price - old_price } else { old_price - price };
                
                if deviation > max_deviation {
                    Self::deposit_event(Event::PriceDeviationDetected {
                        feed_id,
                        old_price,
                        new_price: price,
                    });
                    // Don't fail, but emit warning event
                }
            }

            // Update latest price
            let current_block = frame_system::Pallet::<T>::block_number();
            LatestPrices::<T>::insert(feed_id, (price, current_block));

            // Add to history
            let mut history = PriceHistory::<T>::get(feed_id);
            history.push((price, current_block));
            if history.len() > 100 {
                history.remove(0);  // Keep last 100 prices
            }
            PriceHistory::<T>::insert(feed_id, history);

            // Update feed timestamp
            feed.last_updated = current_block.saturated_into();
            PriceFeeds::<T>::insert(feed_id, feed);

            // Update submission count
            let total = TotalSubmissions::<T>::get();
            TotalSubmissions::<T>::put(total.saturating_add(1));

            Self::deposit_event(Event::PriceSubmitted {
                feed_id,
                price,
                source,
            });

            Ok(())
        }

        /// Add oracle provider to trusted list
        #[pallet::call_index(2)]
        #[pallet::weight(1_000)]
        pub fn add_oracle_provider(
            origin: OriginFor<T>,
            name: Vec<u8>,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;
            TrustedOracles::<T>::insert(&name, true);

            Self::deposit_event(Event::OracleProviderAdded {
                name,
            });

            Ok(())
        }

        /// Disable a price feed
        #[pallet::call_index(3)]
        #[pallet::weight(1_000)]
        pub fn disable_feed(
            origin: OriginFor<T>,
            feed_id: FeedId,
        ) -> DispatchResult {
            let _root = ensure_root(origin)?;

            let mut feed = Self::price_feed(feed_id).ok_or(Error::<T>::FeedNotFound)?;
            feed.enabled = false;
            PriceFeeds::<T>::insert(feed_id, feed);

            Ok(())
        }

        /// Aggregate prices from multiple sources
        #[pallet::call_index(4)]
        #[pallet::weight(20_000)]
        pub fn aggregate_prices(
            origin: OriginFor<T>,
            feed_ids: Vec<FeedId>,
            method: AggregationMethod,
        ) -> DispatchResult {
            let _caller = ensure_signed(origin)?;

            // Verify sufficient sources
            let source_count = feed_ids.len() as u32;
            ensure!(source_count >= T::MinPriceSources::get(), Error::<T>::InsufficientSources);
            ensure!(source_count <= T::MaxPriceSources::get(), Error::<T>::InsufficientSources);

            // Collect prices
            let mut prices = Vec::new();
            for feed_id in &feed_ids {
                if let Some((price, _)) = Self::latest_price(feed_id) {
                    prices.push(price);
                }
            }

            ensure!(prices.len() as u32 >= T::MinPriceSources::get(), Error::<T>::InsufficientSources);

            // Aggregate based on method
            let aggregated_price = match method {
                AggregationMethod::Median => {
                    prices.sort();
                    prices[prices.len() / 2]
                },
                AggregationMethod::TimeWeighted => {
                    let mut weighted_sum = 0u128;
                    for (i, price) in prices.iter().enumerate() {
                        let weight = ((prices.len() - i) as u128) * 1000u128;
                        weighted_sum = weighted_sum.saturating_add(price.saturating_mul(weight));
                    }
                    weighted_sum / (prices.len() as u128 * 1000u128)
                },
                _ => {
                    // Default to median
                    prices.sort();
                    prices[prices.len() / 2]
                },
            };

            Self::deposit_event(Event::PriceAggregated {
                feed_id: feed_ids[0],
                aggregated_price,
                source_count: prices.len() as u32,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn next_feed_id() -> FeedId {
            let counter = FeedCounter::<T>::get();
            let new_counter = counter.saturating_add(1);
            FeedCounter::<T>::put(new_counter);
            new_counter
        }

        pub fn get_price(feed_id: FeedId) -> Option<Price> {
            LatestPrices::<T>::get(feed_id).map(|(price, _)| price)
        }

        pub fn get_price_with_freshness(feed_id: FeedId, max_age: u32) -> Option<Price> {
            if let Some((price, block)) = LatestPrices::<T>::get(feed_id) {
                let current_block = frame_system::Pallet::<T>::block_number().saturated_into::<u32>();
                if current_block.saturating_sub(block.saturated_into()) <= max_age {
                    return Some(price);
                }
            }
            None
        }

        pub fn get_stats() -> (FeedId, u32) {
            (FeedCounter::<T>::get(), TotalSubmissions::<T>::get())
        }
    }
}
