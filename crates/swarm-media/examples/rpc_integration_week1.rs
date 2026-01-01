/// Week 1 RPC Integration Example
///
/// This demonstrates how to use all 6 Media RPC endpoints:
/// 1. media_status         - Get current production status
/// 2. media_schedule       - Get cadence schedule
/// 3. media_contributors   - List active contributors
/// 4. media_metrics        - Get production metrics
/// 5. media_request_repurposing - Submit repurposing job
/// 6. media_job_status     - Track job progress

use swarm_media::{
    MediaRpcHandler, MediaOrchestrationSystem,
    MediaStatusRequest, MediaScheduleRequest, MediaContributorsRequest,
    MediaMetricsRequest, MediaRepurposingRequest, MediaJobStatusRequest,
};
use std::sync::{Arc, Mutex};

#[test]
fn test_rpc_integration_example() {
    // Initialize the media system
    let media = Arc::new(Mutex::new(MediaOrchestrationSystem::default()));
    let handler = MediaRpcHandler::new(media.clone());

    println!("\n═══════════════════════════════════════════════════════");
    println!("  WEEK 1: RPC INTEGRATION - 6 ENDPOINTS EXAMPLE");
    println!("═══════════════════════════════════════════════════════\n");

    // ═════════════════════════════════════════════════════════════
    // ENDPOINT 1: media_status
    // ═════════════════════════════════════════════════════════════
    println!("📊 ENDPOINT 1: media_status");
    println!("─── Get current production status ───\n");

    let status_req = MediaStatusRequest {};
    match handler.media_status(status_req) {
        Ok(response) => {
            let status = &response.status;
            println!("  Recordings Scheduled:  {}", status.recordings_scheduled);
            println!("  Recordings Completed:  {}", status.recordings_completed);
            println!("  On-Time Percentage:    {:.1}%", status.on_time_percentage);
            println!("  Total Assets Created:  {}", status.total_assets_created);
            println!("  Assets Ready:          {}", status.assets_ready);
            println!("  Assets Published:      {}", status.assets_published);
            println!("  Active Contributors:   {}", status.active_contributors);
            println!("  Total Production Hours:{:.1}", status.total_production_hours);
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // ═════════════════════════════════════════════════════════════
    // ENDPOINT 2: media_schedule
    // ═════════════════════════════════════════════════════════════
    println!("\n📅 ENDPOINT 2: media_schedule");
    println!("─── Get cadence schedule and sessions ───\n");

    let schedule_req = MediaScheduleRequest {
        schedule_id: "sched-001".to_string(),
    };
    match handler.media_schedule(schedule_req) {
        Ok(response) => {
            println!("  Schedule ID:          {}", response.schedule_id);
            println!("  Days of Week:         {:?}", response.days_of_week);
            println!("  Time of Day:          {}", response.time_of_day);
            println!("  Timezone:             {}", response.timezone);
            println!("  Sessions This Month:  {}", response.sessions_this_month.len());
            println!("  On-Time Percentage:   {:.1}%", response.on_time_percentage);
            println!("  On-Time Streak:       {} sessions", response.on_time_streak);
            if let Some(next) = &response.next_session {
                println!("  Next Session:         {} (status: {})", next.session_id, next.status);
            } else {
                println!("  Next Session:         None scheduled");
            }
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // ═════════════════════════════════════════════════════════════
    // ENDPOINT 3: media_contributors
    // ═════════════════════════════════════════════════════════════
    println!("\n👥 ENDPOINT 3: media_contributors");
    println!("─── List active contributors ───\n");

    let contributors_req = MediaContributorsRequest {};
    match handler.media_contributors(contributors_req) {
        Ok(response) => {
            println!("  Total Active Contributors: {}", response.total_active);
            println!("  Total Paused:             {}", response.total_paused);
            println!("  Contributor Details:      {} entries", response.contributors.len());
            for contrib in response.contributors.iter().take(3) {
                println!("\n    • {} ({})", contrib.name, contrib.id);
                println!("      Role:    {}", contrib.role);
                println!("      Status:  {}", contrib.status);
                println!("      Active:  {}", contrib.is_active);
            }
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // ═════════════════════════════════════════════════════════════
    // ENDPOINT 4: media_metrics
    // ═════════════════════════════════════════════════════════════
    println!("\n📈 ENDPOINT 4: media_metrics");
    println!("─── Get detailed production metrics ───\n");

    let metrics_req = MediaMetricsRequest {
        period: "week".to_string(),
    };
    match handler.media_metrics(metrics_req) {
        Ok(response) => {
            let summary = &response.summary;
            println!("  Period:                       {}", response.period);
            println!("  Recordings Scheduled:         {}", summary.recordings_scheduled);
            println!("  Recordings Completed:         {}", summary.recordings_completed);
            println!("  On-Time Percentage:           {:.1}%", summary.on_time_percentage);
            println!("  Total Assets Created:         {}", summary.total_assets_created);
            println!("  Average Assets per Recording: {:.1}", summary.average_assets_per_recording);
            println!("  Total Compensation:           ${:.2}", summary.total_compensation);

            println!("\n  Contributor Breakdown:");
            for contrib_metric in response.contributor_breakdown.iter().take(3) {
                println!("\n    • {} ({})", contrib_metric.contributor_name, contrib_metric.contributor_id);
                println!("      Recordings Featured In: {}", contrib_metric.recordings_featured_in);
                println!("      Assets Created:         {}", contrib_metric.assets_created);
                println!("      Compensation Owed:      ${:.2}", contrib_metric.compensation_owed);
            }

            if !summary.highlights.is_empty() {
                println!("\n  Highlights:");
                for highlight in summary.highlights.iter().take(3) {
                    println!("    • {}", highlight);
                }
            }
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // ═════════════════════════════════════════════════════════════
    // ENDPOINT 5: media_request_repurposing
    // ═════════════════════════════════════════════════════════════
    println!("\n🎬 ENDPOINT 5: media_request_repurposing");
    println!("─── Submit repurposing job to queue ───\n");

    let repurposing_req = MediaRepurposingRequest {
        source_id: "src-001".to_string(),
        asset_type: "Clip".to_string(),
        target: "TikTok".to_string(),
        priority: "High".to_string(),
        title: "Founder's Core Insight".to_string(),
        description: "Key moments from latest founder talk".to_string(),
    };
    match handler.media_request_repurposing(repurposing_req) {
        Ok(response) => {
            println!("  ✅ Job Submitted Successfully!");
            println!("  Job ID:                    {}", response.job_id);
            println!("  Status:                    {}", response.status);
            println!("  Estimated Completion:      {} seconds", response.estimated_completion_seconds);

            // ═════════════════════════════════════════════════════════════
            // ENDPOINT 6: media_job_status
            // ═════════════════════════════════════════════════════════════
            println!("\n⏳ ENDPOINT 6: media_job_status");
            println!("─── Track job progress ───\n");

            let job_status_req = MediaJobStatusRequest {
                job_id: response.job_id.clone(),
            };
            match handler.media_job_status(job_status_req) {
                Ok(job_status) => {
                    println!("  Job ID:                    {}", job_status.job_id);
                    println!("  Status:                    {}", job_status.status);
                    println!("  Priority:                  {}", job_status.priority);
                    println!("  Asset Type:                {}", job_status.asset_type);
                    println!("  Target:                    {}", job_status.target);
                    println!("  Progress:                  {}%", job_status.progress_percentage);
                    println!("  Created At:                {}", job_status.created_at);
                    println!("  Last Update:               {}", job_status.last_update);
                    if let Some(est) = job_status.estimated_completion {
                        println!("  Estimated Completion:      {}", est);
                    }
                    if let Some(err) = job_status.error_message {
                        println!("  Error:                     {}", err);
                    }
                }
                Err(e) => println!("  ❌ Error: {}", e),
            }
        }
        Err(e) => println!("  ❌ Error: {}", e),
    }

    // ═════════════════════════════════════════════════════════════
    // SUMMARY
    // ═════════════════════════════════════════════════════════════
    println!("\n═══════════════════════════════════════════════════════");
    println!("  ✅ ALL 6 RPC ENDPOINTS DEMONSTRATED");
    println!("═══════════════════════════════════════════════════════\n");

    println!("  ENDPOINT SUMMARY:");
    println!("  ✓ media_status          - Production status snapshot");
    println!("  ✓ media_schedule        - Recording schedule + consistency");
    println!("  ✓ media_contributors    - Active contributor list");
    println!("  ✓ media_metrics         - Period-based analytics");
    println!("  ✓ media_request_repurposing - Job submission");
    println!("  ✓ media_job_status      - Job tracking\n");

    println!("  INTEGRATION NOTES:");
    println!("  • These endpoints map to Substrate RPC layer");
    println!("  • In production, calls go through JSON-RPC");
    println!("  • Dashboard consumes these endpoints in real-time");
    println!("  • Job queue receives requests from media_request_repurposing");
    println!("  • All requests/responses are type-safe (serde)\n");

    println!("  NEXT STEPS (Week 2-4):");
    println!("  1. Wire these to Substrate node RPC (node/src/rpc.rs)");
    println!("  2. Create TypeScript types in dashboard");
    println!("  3. Build MediaProductionPanel UI component");
    println!("  4. Implement job queue integration");
    println!("  5. Add payment automation\n");
}
