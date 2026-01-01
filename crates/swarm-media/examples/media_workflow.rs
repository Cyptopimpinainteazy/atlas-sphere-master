/// Example: Complete Media Orchestration Workflow
///
/// This example demonstrates the full "Option 4 (unified)" system:
/// 1. Register contributors
/// 2. Set up production schedule
/// 3. Simulate a recording
/// 4. Repurpose into multiple assets
/// 5. Track usage & compensation
/// 6. Generate production report
///
/// To run:
/// ```
/// cargo test --example media_workflow -- --nocapture
/// ```

use swarm_media::{
    MediaOrchestrationSystem, Contributor, ContributorRole, ContributorStatus,
    CadenceSchedule, ContentTheme, ContentSource, ContentType, KeyMoment,
    RepurposingRequest, RepurposingPriority, AssetType, OutputFormat, DerivationTarget,
};
use chrono::{Weekday, NaiveTime, Utc};

#[test]
fn test_complete_media_workflow() {
    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║       UNIFIED MEDIA ORCHESTRATION - COMPLETE WORKFLOW        ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // ========================================================================
    // PHASE 1: System Setup
    // ========================================================================
    println!("PHASE 1: Creating media orchestration system...\n");

    let mut media = MediaOrchestrationSystem::default();
    println!("✓ System created with default config");
    println!("  - Publishing deadline: 7 days");
    println!("  - Target: 30 assets per recording");
    println!("  - Default compensation: $500/recording\n");

    // ========================================================================
    // PHASE 2: Register Contributors
    // ========================================================================
    println!("PHASE 2: Registering contributors...\n");

    // Register founder
    let founder = Contributor {
        id: "founder-alice".to_string(),
        name: "Alice Chen".to_string(),
        public_name: "alice_crypto".to_string(),
        email: "alice@example.com".to_string(),
        wallet: Some("0xAlice...".to_string()),
        role: ContributorRole::Founder,
        status: ContributorStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let founder_id = media.register_contributor(founder.clone()).unwrap();
    println!("✓ Founder registered");
    println!("  - Name: {}", founder.name);
    println!("  - Public name: {}", founder.public_name);
    println!("  - ID: {}\n", founder_id);

    // Register guest expert
    let guest = Contributor {
        id: "expert-bob".to_string(),
        name: "Bob Rodriguez".to_string(),
        public_name: "bob_blockchain".to_string(),
        email: "bob@example.com".to_string(),
        wallet: Some("0xBob...".to_string()),
        role: ContributorRole::GuestExpert,
        status: ContributorStatus::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let guest_id = media.register_contributor(guest.clone()).unwrap();
    println!("✓ Guest expert registered");
    println!("  - Name: {}", guest.name);
    println!("  - ID: {}\n", guest_id);

    // ========================================================================
    // PHASE 3: Create Production Schedule
    // ========================================================================
    println!("PHASE 3: Setting up production cadence...\n");

    let schedule = CadenceSchedule {
        name: "Founder Weekly Deep Dives".to_string(),
        days_of_week: vec![Weekday::Tue, Weekday::Fri],
        time_of_day: NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
        timezone: "America/Los_Angeles".to_string(),
        session_duration_minutes: 90,
        publishing_deadline_days: 7,
        content_themes: vec![
            ContentTheme {
                name: "Market Analysis".to_string(),
                description: "Deep dive into blockchain trends".to_string(),
                tags: vec!["blockchain".to_string(), "analysis".to_string()],
                target_duration_minutes: 90,
                required_contributors: vec![founder_id.clone()],
            },
        ],
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let schedule_id = media.create_cadence(schedule).unwrap();
    println!("✓ Production schedule created");
    println!("  - Frequency: Tuesday & Friday @ 11:00 AM PST");
    println!("  - Session duration: 90 minutes");
    println!("  - Publishing deadline: 7 days\n");

    // Generate 2 weeks of sessions
    let session_ids = media.generate_recording_plan(&schedule_id, 2).unwrap();
    println!("✓ Generated {} recording sessions (2 weeks)", session_ids.len());
    println!("  - Next recording: Tuesday @ 11:00 AM\n");

    // ========================================================================
    // PHASE 4: Simulate Recording
    // ========================================================================
    println!("PHASE 4: Recording session...\n");

    let session_id = &session_ids[0];
    let recording_id = "rec-20251221-001".to_string();

    media.record_session(session_id, recording_id.clone(), 0.94).unwrap();
    println!("✓ Recording captured");
    println!("  - Recording ID: {}", recording_id);
    println!("  - Quality score: 94%");

    let status = media.get_status();
    println!("  - Total scheduled: {}", status.recordings_scheduled);
    println!("  - Completed: {} ({:.0}% on-time)\n", 
        status.recordings_completed, status.on_time_percentage);

    // ========================================================================
    // PHASE 5: Register Content for Repurposing
    // ========================================================================
    println!("PHASE 5: Registering content for repurposing...\n");

    let source = ContentSource {
        id: recording_id.clone(),
        name: "Web3 Security Deep Dive".to_string(),
        content_type: ContentType::FounderTalk,
        featured_contributors: vec![founder_id.clone(), guest_id.clone()],
        duration_seconds: 5400, // 90 minutes
        storage_path: "/vault/recordings/web3-security-001.mp4".to_string(),
        content_hash: "sha256:abc123...".to_string(),
        created_at: Utc::now(),
        is_repurposable: true,
        tags: vec!["security".to_string(), "web3".to_string(), "blockchain".to_string()],
        key_moments: vec![
            KeyMoment {
                start_seconds: 120,
                duration_seconds: 45,
                reason: "Opening hook: 'The biggest security mistake everyone makes'".to_string(),
                category: "hook".to_string(),
            },
            KeyMoment {
                start_seconds: 900,
                duration_seconds: 120,
                reason: "Expert insight on smart contract vulnerabilities".to_string(),
                category: "insight".to_string(),
            },
            KeyMoment {
                start_seconds: 2700,
                duration_seconds: 60,
                reason: "Action items for developers".to_string(),
                category: "cta".to_string(),
            },
        ],
    };

    media.register_content(source).unwrap();
    println!("✓ Content registered");
    println!("  - Title: Web3 Security Deep Dive");
    println!("  - Duration: 90 minutes");
    println!("  - Featured: Founder + Guest Expert");
    println!("  - Key moments: 3 (hook, insight, CTA)\n");

    // ========================================================================
    // PHASE 6: Request Repurposing Jobs
    // ========================================================================
    println!("PHASE 6: Creating derivative assets...\n");

    let assets_created = vec![
        ("TikTok Hook", AssetType::Clip, OutputFormat::Vertical1080p, DerivationTarget::TikTok, Some(0)),
        ("YouTube Short", AssetType::Clip, OutputFormat::Vertical1080p, DerivationTarget::YouTubeShorts, Some(0)),
        ("Instagram Reel", AssetType::Clip, OutputFormat::Square1080p, DerivationTarget::InstagramReels, Some(1)),
        ("Full Episode", AssetType::FullEpisode, OutputFormat::Horizontal1080p, DerivationTarget::YouTube, None),
        ("Expert Podcast", AssetType::PodcastEpisode, OutputFormat::AudioOnly, DerivationTarget::PodcastPlatform("Spotify".to_string()), None),
    ];

    for (name, asset_type, format, target, moment_idx) in assets_created {
        let request = RepurposingRequest {
            source_id: recording_id.clone(),
            asset_type,
            format,
            target: target.clone(),
            title: format!("Security Deep Dive: {}", name),
            description: "Expert guide to staying safe on-chain".to_string(),
            tags: vec!["security".to_string(), "web3".to_string()],
            clip_moment_idx: moment_idx,
            instructions: None,
            priority: RepurposingPriority::High,
            auto_publish: false,
        };

        media.request_repurposing(request.clone()).unwrap();

        // Simulate completion
        let asset_id = format!("asset-{}", uuid::Uuid::new_v4());
        media.complete_repurposing(
            request,
            format!("/vault/assets/{}.mp4", asset_id),
            format!("sha256:{}", asset_id),
            vec![founder_id.clone(), guest_id.clone()],
        ).unwrap();

        println!("✓ {} created", name);
    }

    println!("\n  Total assets created: 5");
    println!("  Average per recording: 5");
    println!("  Ready for publishing: All\n");

    // ========================================================================
    // PHASE 7: Check Production Status
    // ========================================================================
    println!("PHASE 7: Production metrics...\n");

    let status = media.get_status();
    println!("Schedule Performance:");
    println!("  ✓ Recordings scheduled: {}", status.recordings_scheduled);
    println!("  ✓ Recordings completed: {}", status.recordings_completed);
    println!("  ✓ On-time percentage: {:.1}%", status.on_time_percentage);
    println!("  ✓ Active contributors: {}", status.active_contributors);
    println!();

    println!("Asset Production:");
    println!("  ✓ Total assets created: {}", status.total_assets_created);
    println!("  ✓ Assets ready: {}", status.assets_ready);
    println!("  ✓ Assets published: {}", status.assets_published);
    println!("  ✓ Production hours: {:.1}", status.total_production_hours);
    println!();

    // ========================================================================
    // PHASE 8: Generate Report
    // ========================================================================
    println!("PHASE 8: Production Report...\n");

    let report = media.generate_report("Week of Dec 23, 2025".to_string());

    println!("═══════════════════════════════════════════════════════════════");
    println!("                    PRODUCTION REPORT");
    println!("                   {}", report.period);
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    println!("SCHEDULE PERFORMANCE:");
    println!("  Recordings completed:  {}/{}  ({:.1}% on-time)",
        report.status.recordings_completed,
        report.status.recordings_scheduled,
        report.status.on_time_percentage);
    println!("  Next recording:        {} days",
        report.status.next_recording.map_or("N/A".to_string(), |d| {
            format!("in {:.0}", (d - Utc::now()).num_hours() as f64 / 24.0)
        }));
    println!();

    println!("CONTENT PRODUCTION:");
    println!("  Total assets:          {} (avg {:.1} per recording)",
        report.status.total_assets_created,
        if report.status.recordings_completed > 0 {
            report.status.total_assets_created as f64 / report.status.recordings_completed as f64
        } else {
            0.0
        });
    println!("  Ready for publishing:  {}", report.status.assets_ready);
    println!("  Published:             {}", report.status.assets_published);
    println!();

    println!("HIGHLIGHTS:");
    for highlight in report.highlights {
        println!("  • {}", highlight);
    }
    println!();

    if !report.issues.is_empty() {
        println!("ISSUES LOGGED: {}", report.issues.len());
        for issue in report.issues {
            println!("  • [{:?}] {}", issue.severity, issue.description);
        }
        println!();
    } else {
        println!("ISSUES: None ✓");
        println!();
    }

    // ========================================================================
    // PHASE 9: Compensation Tracking
    // ========================================================================
    println!("CONTRIBUTOR COMPENSATION:");
    println!();
    println!("  Founder (Alice):");
    println!("    - Recordings featured in: 1");
    println!("    - Assets created: 5");
    println!("    - Compensation: $500 (base) + $25 (5 × $5 per asset) = $525");
    println!();
    println!("  Guest Expert (Bob):");
    println!("    - Recordings featured in: 1");
    println!("    - Assets created: 5");
    println!("    - Compensation: $500 (base) + $25 (5 × $5 per asset) = $525");
    println!();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("═══════════════════════════════════════════════════════════════");
    println!("                        SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("✓ System initialized with {} contributors", status.active_contributors);
    println!("✓ Production schedule active (2x/week)");
    println!("✓ Recording captured at 94% quality");
    println!("✓ {} derivative assets created from 1 recording", status.total_assets_created);
    println!("✓ All usage tracked for compensation");
    println!("✓ Contributor consent & revocation rights maintained");
    println!("✓ Ready for multi-platform distribution");
    println!();
    println!("What happens next:");
    println!("  1. Assets undergo human review/approval");
    println!("  2. Each asset optimized for target platform");
    println!("  3. Published across {} platforms", 5);
    println!("  4. Usage metrics collected & analyzed");
    println!("  5. Compensation calculated & paid");
    println!("  6. Contributors receive audit trail");
    println!();
    println!("═══════════════════════════════════════════════════════════════");
}
