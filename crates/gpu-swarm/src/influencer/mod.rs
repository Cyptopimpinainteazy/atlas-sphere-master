//! AI Influencer System - Hyper-Realistic Social Media Personas
//!
//! This module implements the complete AI influencer infrastructure for Atlas Sphere,
//! enabling autonomous, convincing social media presence across all major platforms.
//!
//! # Core Components
//!
//! - **PersonaGenerator**: Creates hyper-realistic AI personas with consistent identities
//! - **AvatarGenerator**: Generates photorealistic profile images using diffusion models
//! - **ProfileManager**: Manages profile data across all social platforms
//! - **ContentEngine**: Produces marketing content (images, videos, slideshows)
//! - **SocialPlatformManager**: Handles posting, engagement, and community interaction
//! - **InfluencerDiscovery**: Finds and engages with crypto influencers and communities
//!
//! # Security & Compliance
//!
//! - All AI content is labeled for transparency
//! - Platform ToS compliance monitoring
//! - Rate limiting to prevent spam detection
//! - Audit logging for all actions

pub mod avatar;
pub mod content;
pub mod discovery;
pub mod persona;
pub mod platforms;
pub mod profile;

pub use avatar::{AvatarConfig, AvatarGenerator, AvatarStyle, GeneratedAvatar};
pub use content::{
    ContentEngine, ContentType, GeneratedContent, ImageConfig, SlideshowConfig, VideoConfig,
};
pub use discovery::{
    CryptoInfluencer, DiscoveryConfig, EngagementStrategy, InfluencerDiscovery, InfluencerType,
};
pub use persona::{
    AIPersona, PersonaConfig, PersonaGenerator, PersonaGender, PersonaStyle, PersonaTrait,
};
pub use platforms::{
    EngagementAction, PlatformAccount, PlatformConfig, PlatformManager, PostConfig, PostResult,
    SocialPlatform,
};
pub use profile::{InfluencerProfile, ProfileManager, ProfileStatus, SocialProfile};
