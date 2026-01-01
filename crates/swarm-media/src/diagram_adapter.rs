//! Diagram & Data Visualization Adapter for Swarm Media
//!
//! Provides automated chart and diagram generation with support for:
//! - D3.js/Vega-Lite for data visualizations
//! - Mermaid for flowcharts and sequence diagrams
//! - PlantUML for architecture diagrams
//! - Custom chart templates for DeFi metrics
//!
//! Task 8 from ARCHITECTURE_COMPLETE.md

use crate::tool_adapter::{GpuNodeCapabilities, JobId, JobStatus, Priority, ToolAdapter, ToolParams, ToolResourceReq, ToolResult, ToolType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Supported diagram/chart types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramType {
    /// Line chart for time-series data
    LineChart,
    /// Bar chart for comparisons
    BarChart,
    /// Pie/donut chart for proportions
    PieChart,
    /// Candlestick chart for DeFi trading data
    CandlestickChart,
    /// Area chart for stacked values
    AreaChart,
    /// Flowchart (Mermaid-based)
    Flowchart,
    /// Sequence diagram (Mermaid-based)
    SequenceDiagram,
    /// Architecture diagram (PlantUML-based)
    ArchitectureDiagram,
    /// Entity relationship diagram
    ERDiagram,
    /// Network topology diagram
    NetworkDiagram,
    /// Sankey diagram for flow visualization
    SankeyDiagram,
    /// Treemap for hierarchical data
    Treemap,
    /// Custom Vega-Lite specification
    CustomVegaLite,
}

impl Default for DiagramType {
    fn default() -> Self {
        DiagramType::LineChart
    }
}

/// Output format for diagrams
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramOutputFormat {
    /// SVG (scalable, best for web)
    Svg,
    /// PNG (raster, best for social media)
    Png,
    /// PDF (for documents)
    Pdf,
    /// Interactive HTML
    Html,
}

impl Default for DiagramOutputFormat {
    fn default() -> Self {
        DiagramOutputFormat::Svg
    }
}

/// Data source for charts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataSource {
    /// Inline JSON data
    Inline(serde_json::Value),
    /// URL to fetch data from
    Url(String),
    /// SQL query against indexed data
    Query(String),
    /// On-chain data (block range, contract address)
    OnChain {
        chain: String,
        address: String,
        method: String,
        from_block: Option<u64>,
        to_block: Option<u64>,
    },
}

/// Theme for diagram styling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramTheme {
    /// Primary color (hex)
    pub primary_color: String,
    /// Secondary color (hex)
    pub secondary_color: String,
    /// Background color (hex)
    pub background_color: String,
    /// Text color (hex)
    pub text_color: String,
    /// Font family
    pub font_family: String,
    /// Font size in pixels
    pub font_size: u32,
    /// Enable grid lines
    pub show_grid: bool,
    /// Enable animations (HTML output only)
    pub animated: bool,
}

impl Default for DiagramTheme {
    fn default() -> Self {
        DiagramTheme {
            primary_color: "#3B82F6".to_string(),    // Blue
            secondary_color: "#8B5CF6".to_string(),   // Purple
            background_color: "#1F2937".to_string(),  // Dark gray
            text_color: "#F3F4F6".to_string(),        // Light gray
            font_family: "Inter, system-ui, sans-serif".to_string(),
            font_size: 12,
            show_grid: true,
            animated: false,
        }
    }
}

/// Diagram generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramParams {
    /// Type of diagram to generate
    pub diagram_type: DiagramType,
    /// Title for the diagram
    pub title: Option<String>,
    /// Subtitle/description
    pub subtitle: Option<String>,
    /// Data source
    pub data: DataSource,
    /// Output format
    pub output_format: DiagramOutputFormat,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Theme/styling options
    pub theme: DiagramTheme,
    /// Additional options (type-specific)
    pub options: Option<serde_json::Value>,
    /// Mermaid/PlantUML source code (for diagram types)
    pub source_code: Option<String>,
}

impl Default for DiagramParams {
    fn default() -> Self {
        DiagramParams {
            diagram_type: DiagramType::default(),
            title: None,
            subtitle: None,
            data: DataSource::Inline(serde_json::json!([])),
            output_format: DiagramOutputFormat::default(),
            width: 800,
            height: 600,
            theme: DiagramTheme::default(),
            options: None,
            source_code: None,
        }
    }
}

/// Result of diagram generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramResult {
    /// URL/path to generated diagram
    pub diagram_url: String,
    /// Thumbnail URL (if generated)
    pub thumbnail_url: Option<String>,
    /// Diagram dimensions
    pub width: u32,
    pub height: u32,
    /// Output format used
    pub format: DiagramOutputFormat,
    /// Content hash for caching
    pub content_hash: String,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
    /// Vega-Lite spec used (if applicable)
    pub spec: Option<serde_json::Value>,
}

/// Diagram job tracking
#[derive(Debug, Clone)]
struct DiagramJob {
    job_id: JobId,
    params: DiagramParams,
    status: JobStatus,
    created_at: i64,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    result: Option<DiagramResult>,
    assigned_node: Option<Uuid>,
}

/// Diagram Adapter Configuration
#[derive(Debug, Clone)]
pub struct DiagramAdapterConfig {
    /// Vega-Lite renderer URL (local or cloud)
    pub vega_renderer_url: String,
    /// Mermaid CLI server URL
    pub mermaid_server_url: String,
    /// PlantUML server URL
    pub plantuml_server_url: String,
    /// Output directory for generated diagrams
    pub output_dir: String,
    /// Maximum resolution (width * height)
    pub max_resolution: u32,
    /// Enable caching
    pub enable_cache: bool,
}

impl Default for DiagramAdapterConfig {
    fn default() -> Self {
        DiagramAdapterConfig {
            vega_renderer_url: "http://localhost:8004".to_string(),
            mermaid_server_url: "http://localhost:8005".to_string(),
            plantuml_server_url: "http://localhost:8006".to_string(),
            output_dir: "/var/cache/diagrams".to_string(),
            max_resolution: 4096 * 4096,
            enable_cache: true,
        }
    }
}

/// Diagram Generation Adapter
///
/// Implements the ToolAdapter trait for automated chart and diagram generation.
pub struct DiagramAdapter {
    config: DiagramAdapterConfig,
    jobs: Arc<RwLock<HashMap<JobId, DiagramJob>>>,
    content_cache: Arc<RwLock<HashMap<String, DiagramResult>>>,
    templates: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl DiagramAdapter {
    pub fn new(config: DiagramAdapterConfig) -> Self {
        let templates = Self::load_builtin_templates();

        DiagramAdapter {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            content_cache: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(templates)),
        }
    }

    /// Load built-in Vega-Lite templates
    fn load_builtin_templates() -> HashMap<String, serde_json::Value> {
        let mut templates = HashMap::new();

        // DeFi price chart template
        templates.insert("defi_price_chart".to_string(), serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
            "mark": {"type": "line", "point": true},
            "encoding": {
                "x": {"field": "timestamp", "type": "temporal", "title": "Time"},
                "y": {"field": "price", "type": "quantitative", "title": "Price (USD)"},
                "color": {"field": "token", "type": "nominal"}
            }
        }));

        // TVL chart template
        templates.insert("tvl_chart".to_string(), serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
            "mark": "area",
            "encoding": {
                "x": {"field": "date", "type": "temporal"},
                "y": {"field": "tvl", "type": "quantitative", "stack": "zero"},
                "color": {"field": "protocol", "type": "nominal"}
            }
        }));

        // Transaction volume bar chart
        templates.insert("tx_volume_chart".to_string(), serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
            "mark": "bar",
            "encoding": {
                "x": {"field": "date", "type": "temporal", "timeUnit": "yearmonthdate"},
                "y": {"field": "volume", "type": "quantitative"},
                "color": {"value": "#3B82F6"}
            }
        }));

        // Network topology template
        templates.insert("network_topology".to_string(), serde_json::json!({
            "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
            "layer": [
                {
                    "mark": {"type": "rule"},
                    "encoding": {
                        "x": {"field": "source_x", "type": "quantitative"},
                        "y": {"field": "source_y", "type": "quantitative"},
                        "x2": {"field": "target_x"},
                        "y2": {"field": "target_y"}
                    }
                },
                {
                    "mark": {"type": "circle", "size": 200},
                    "encoding": {
                        "x": {"field": "x", "type": "quantitative"},
                        "y": {"field": "y", "type": "quantitative"},
                        "color": {"field": "type", "type": "nominal"}
                    }
                }
            ]
        }));

        templates
    }

    /// Compute content hash for caching
    fn compute_content_hash(params: &DiagramParams) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", params.diagram_type).as_bytes());
        hasher.update(format!("{:?}", params.data).as_bytes());
        hasher.update(&params.width.to_le_bytes());
        hasher.update(&params.height.to_le_bytes());
        hasher.update(format!("{:?}", params.theme).as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generate Vega-Lite specification
    fn generate_vega_spec(&self, params: &DiagramParams) -> Result<serde_json::Value, String> {
        let base_spec = match params.diagram_type {
            DiagramType::LineChart => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": {"type": "line", "point": true, "strokeWidth": 2},
            }),
            DiagramType::BarChart => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": {"type": "bar", "cornerRadiusTopLeft": 4, "cornerRadiusTopRight": 4},
            }),
            DiagramType::PieChart => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": {"type": "arc", "innerRadius": 50},
            }),
            DiagramType::AreaChart => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": {"type": "area", "opacity": 0.7},
            }),
            DiagramType::CandlestickChart => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "layer": [
                    {
                        "mark": {"type": "rule"},
                        "encoding": {
                            "y": {"field": "low", "type": "quantitative"},
                            "y2": {"field": "high"}
                        }
                    },
                    {
                        "mark": {"type": "bar", "size": 10},
                        "encoding": {
                            "y": {"field": "open", "type": "quantitative"},
                            "y2": {"field": "close"},
                            "color": {
                                "condition": {"test": "datum.open < datum.close", "value": "#22C55E"},
                                "value": "#EF4444"
                            }
                        }
                    }
                ]
            }),
            DiagramType::SankeyDiagram => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega/v5.json",
                "marks": [{"type": "rect"}, {"type": "path"}]
            }),
            DiagramType::Treemap => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": "rect",
                "transform": [{"type": "treemap", "field": "value"}]
            }),
            _ => serde_json::json!({
                "$schema": "https://vega.github.io/schema/vega-lite/v5.json",
                "mark": "point",
            }),
        };

        // Apply theme
        let theme = &params.theme;
        let mut spec = serde_json::json!({
            "width": params.width,
            "height": params.height,
            "title": params.title,
            "config": {
                "background": theme.background_color,
                "title": {"color": theme.text_color, "font": theme.font_family},
                "axis": {
                    "labelColor": theme.text_color,
                    "titleColor": theme.text_color,
                    "gridColor": "#374151",
                    "grid": theme.show_grid
                },
                "legend": {"labelColor": theme.text_color, "titleColor": theme.text_color},
                "mark": {"color": theme.primary_color}
            },
            "data": params.data.clone()
        });
        
        // Merge base_spec properties
        if let Some(obj) = spec.as_object_mut() {
            if let Some(base_obj) = base_spec.as_object() {
                for (k, v) in base_obj.iter() {
                    if !obj.contains_key(k) {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        Ok(spec)
    }

    /// Render diagram using Vega-Lite renderer
    async fn render_vega(&self, spec: &serde_json::Value, format: &DiagramOutputFormat) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "spec": spec,
            "format": match format {
                DiagramOutputFormat::Svg => "svg",
                DiagramOutputFormat::Png => "png",
                DiagramOutputFormat::Pdf => "pdf",
                DiagramOutputFormat::Html => "html",
            }
        });

        let response = client
            .post(&format!("{}/render", self.config.vega_renderer_url))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Vega renderer request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Vega renderer error: {}", response.status()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read render output: {}", e))
    }

    /// Render Mermaid diagram
    async fn render_mermaid(&self, source: &str, format: &DiagramOutputFormat) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "code": source,
            "format": match format {
                DiagramOutputFormat::Svg => "svg",
                DiagramOutputFormat::Png => "png",
                _ => "svg",
            },
            "theme": "dark"
        });

        let response = client
            .post(&format!("{}/render", self.config.mermaid_server_url))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Mermaid renderer request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Mermaid renderer error: {}", response.status()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read Mermaid output: {}", e))
    }

    /// Render PlantUML diagram
    async fn render_plantuml(&self, source: &str, format: &DiagramOutputFormat) -> Result<Vec<u8>, String> {
        let client = reqwest::Client::new();

        // PlantUML uses base64-encoded deflated source
        let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        encoder
            .write_all(source.as_bytes())
            .map_err(|e| format!("Failed to deflate PlantUML source: {}", e))?;
        let encoded_payload = encoder
            .finish()
            .map_err(|e| format!("Failed to finalize PlantUML payload: {}", e))?;
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            encoded_payload,
        );

        let format_ext = match format {
            DiagramOutputFormat::Svg => "svg",
            DiagramOutputFormat::Png => "png",
            _ => "svg",
        };

        let response = client
            .post(&format!("{}/{}/{}", self.config.plantuml_server_url, format_ext, encoded))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("PlantUML renderer request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("PlantUML renderer error: {}", response.status()));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| format!("Failed to read PlantUML output: {}", e))
    }

    /// Generate diagram based on type
    async fn generate_diagram(&self, params: &DiagramParams) -> Result<DiagramResult, String> {
        // Check cache first
        let cache_key = Self::compute_content_hash(params);
        if self.config.enable_cache {
            let cache = self.content_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let start_time = std::time::Instant::now();

        let (rendered_bytes, spec) = match params.diagram_type {
            DiagramType::Flowchart | DiagramType::SequenceDiagram | DiagramType::ERDiagram => {
                let source = params.source_code.as_ref()
                    .ok_or("Mermaid source code required for diagram type")?;
                (self.render_mermaid(source, &params.output_format).await?, None)
            }
            DiagramType::ArchitectureDiagram | DiagramType::NetworkDiagram => {
                let source = params.source_code.as_ref()
                    .ok_or("PlantUML source code required for diagram type")?;
                (self.render_plantuml(source, &params.output_format).await?, None)
            }
            _ => {
                let spec = self.generate_vega_spec(params)?;
                let bytes = self.render_vega(&spec, &params.output_format).await?;
                (bytes, Some(spec))
            }
        };

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        // Save to file
        let file_ext = match params.output_format {
            DiagramOutputFormat::Svg => "svg",
            DiagramOutputFormat::Png => "png",
            DiagramOutputFormat::Pdf => "pdf",
            DiagramOutputFormat::Html => "html",
        };
        
        let output_path = format!("{}/diagram_{}.{}", self.config.output_dir, Uuid::new_v4(), file_ext);
        
        // Ensure output directory exists
        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        
        tokio::fs::write(&output_path, &rendered_bytes)
            .await
            .map_err(|e| format!("Failed to save diagram: {}", e))?;

        let result = DiagramResult {
            diagram_url: output_path,
            thumbnail_url: None, // TODO: Generate thumbnail for large diagrams
            width: params.width,
            height: params.height,
            format: params.output_format.clone(),
            content_hash: cache_key.clone(),
            generation_time_ms,
            spec,
        };

        // Cache result
        if self.config.enable_cache {
            let mut cache = self.content_cache.write().await;
            cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    /// Register a custom template
    pub async fn register_template(&self, name: &str, spec: serde_json::Value) -> Result<(), String> {
        let mut templates = self.templates.write().await;
        templates.insert(name.to_string(), spec);
        Ok(())
    }

    /// Get template by name
    pub async fn get_template(&self, name: &str) -> Option<serde_json::Value> {
        let templates = self.templates.read().await;
        templates.get(name).cloned()
    }

    /// List available templates
    pub async fn list_templates(&self) -> Vec<String> {
        let templates = self.templates.read().await;
        templates.keys().cloned().collect()
    }
}

#[async_trait]
impl ToolAdapter for DiagramAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::DiagramGeneration
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        let diagram_params: DiagramParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid diagram params: {}", e))?;

        // Check resolution limits
        let resolution = diagram_params.width as u64 * diagram_params.height as u64;
        if resolution > self.config.max_resolution as u64 {
            return Err(format!(
                "Resolution {}x{} exceeds maximum of {} pixels",
                diagram_params.width,
                diagram_params.height,
                self.config.max_resolution
            ));
        }

        // Check for required source code for Mermaid/PlantUML types
        match diagram_params.diagram_type {
            DiagramType::Flowchart | DiagramType::SequenceDiagram | 
            DiagramType::ERDiagram | DiagramType::ArchitectureDiagram |
            DiagramType::NetworkDiagram => {
                if diagram_params.source_code.is_none() {
                    return Err("Source code required for this diagram type".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let diagram_params: DiagramParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid diagram params: {}", e))?;

        let job_id = Uuid::new_v4();
        let job = DiagramJob {
            job_id,
            params: diagram_params,
            status: JobStatus::Queued,
            created_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            result: None,
            assigned_node: None,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id, job);
        }

        // Execute in background
        let adapter = self.clone();
        let job_id_clone = job_id;
        tokio::spawn(async move {
            adapter.execute_job(job_id_clone).await;
        });

        Ok(job_id)
    }

    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id)
            .map(|j| j.status.clone())
            .ok_or_else(|| format!("Job {} not found", job_id))
    }

    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String> {
        let jobs = self.jobs.read().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| format!("Job {} not found", job_id))?;

        match &job.status {
            JobStatus::Completed => {
                let result = job.result.as_ref()
                    .ok_or("Result not available")?;
                
                Ok(ToolResult {
                    job_id,
                    tool_type: ToolType::DiagramGeneration,
                    output: serde_json::to_value(result).unwrap(),
                    execution_time_ms: result.generation_time_ms as u32,
                    content_hash: Some(result.content_hash.clone()),
                    executed_by_node: job.assigned_node.unwrap_or(Uuid::nil()),
                })
            }
            JobStatus::Failed(err) => Err(format!("Job failed: {}", err)),
            _ => Err("Job not yet completed".to_string()),
        }
    }

    async fn cancel_job(&self, job_id: JobId) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            match job.status {
                JobStatus::Queued | JobStatus::Assigned => {
                    job.status = JobStatus::Cancelled;
                    Ok(())
                }
                JobStatus::Running => Err("Cannot cancel running job".to_string()),
                _ => Err("Job already completed or cancelled".to_string()),
            }
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }

    fn resource_requirements(&self, _params: &ToolParams) -> ToolResourceReq {
        // Diagram generation is CPU-bound, no GPU needed
        ToolResourceReq {
            min_vram_gb: 0,
            preferred_latency_ms: 2000,
            supports_batching: true,
        }
    }
}

impl DiagramAdapter {
    async fn execute_job(&self, job_id: JobId) {
        // Update status to running
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(chrono::Utc::now().timestamp());
            }
        }

        // Get job params
        let params = {
            let jobs = self.jobs.read().await;
            jobs.get(&job_id).map(|j| j.params.clone())
        };

        let Some(params) = params else {
            return;
        };

        // Execute generation
        let result = self.generate_diagram(&params).await;

        // Update job with result
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match result {
                    Ok(diagram_result) => {
                        job.status = JobStatus::Completed;
                        job.result = Some(diagram_result);
                    }
                    Err(err) => {
                        job.status = JobStatus::Failed(err);
                    }
                }
                job.completed_at = Some(chrono::Utc::now().timestamp());
            }
        }
    }
}

impl Clone for DiagramAdapter {
    fn clone(&self) -> Self {
        DiagramAdapter {
            config: self.config.clone(),
            jobs: Arc::clone(&self.jobs),
            content_cache: Arc::clone(&self.content_cache),
            templates: Arc::clone(&self.templates),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagram_params_default() {
        let params = DiagramParams::default();
        assert_eq!(params.width, 800);
        assert_eq!(params.height, 600);
        assert_eq!(params.output_format, DiagramOutputFormat::Svg);
    }

    #[test]
    fn test_theme_default() {
        let theme = DiagramTheme::default();
        assert_eq!(theme.primary_color, "#3B82F6");
        assert!(theme.show_grid);
    }

    #[test]
    fn test_content_hash_consistency() {
        let params1 = DiagramParams {
            title: Some("Test Chart".to_string()),
            ..Default::default()
        };
        let params2 = DiagramParams {
            title: Some("Test Chart".to_string()),
            ..Default::default()
        };

        let hash1 = DiagramAdapter::compute_content_hash(&params1);
        let hash2 = DiagramAdapter::compute_content_hash(&params2);

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        let config = DiagramAdapterConfig::default();
        let adapter = DiagramAdapter::new(config);

        let templates = adapter.list_templates().await;
        assert!(!templates.is_empty());
        assert!(templates.contains(&"defi_price_chart".to_string()));
    }

    #[tokio::test]
    async fn test_param_validation() {
        let config = DiagramAdapterConfig::default();
        let adapter = DiagramAdapter::new(config);

        // Valid params should pass
        let params = ToolParams::new(serde_json::json!({
            "diagram_type": "LineChart",
            "width": 800,
            "height": 600,
            "data": {"values": [{"x": 1, "y": 2}]}
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_ok());

        // Oversized resolution should fail
        let params = ToolParams::new(serde_json::json!({
            "diagram_type": "LineChart",
            "width": 10000,
            "height": 10000,
            "data": {"values": []}
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vega_spec_generation() {
        let config = DiagramAdapterConfig::default();
        let adapter = DiagramAdapter::new(config);

        let params = DiagramParams {
            diagram_type: DiagramType::LineChart,
            title: Some("Test".to_string()),
            width: 800,
            height: 600,
            ..Default::default()
        };

        let spec = adapter.generate_vega_spec(&params);
        assert!(spec.is_ok());
        
        let spec = spec.unwrap();
        assert_eq!(spec["width"], 800);
        assert_eq!(spec["height"], 600);
    }
}
