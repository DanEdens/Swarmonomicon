use anyhow::Result;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use plotters::prelude::*;
use crate::agents::rl::model::config::{TrainingHistory, TrainingMetrics};

#[derive(Clone)]
pub struct VisualizationTools {
    output_dir: PathBuf,
}

impl VisualizationTools {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        let path = PathBuf::from(output_dir.as_ref());
        std::fs::create_dir_all(&path).unwrap_or_default();
        
        Self { output_dir: path }
    }

    /// Generate reward plot from training history
    pub fn plot_rewards(&self, history: &TrainingHistory) -> Result<PathBuf> {
        let output_path = self.output_dir.join("rewards.png");
        
        {
            let root = BitMapBackend::new(&output_path, (800, 600)).into_drawing_area();
            root.fill(&WHITE)?;

            let episodes: Vec<usize> = history.metrics.iter().map(|m| m.episode).collect();
            let rewards: Vec<f64> = history.metrics.iter().map(|m| m.reward).collect();
            
            let min_e = episodes.iter().min().copied().unwrap_or(0);
            let max_e = episodes.iter().max().copied().unwrap_or(100);
            let min_r = rewards.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(0.0);
            let max_r = rewards.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(1.0);

            let mut chart = ChartBuilder::on(&root)
                .caption("Training Rewards", ("sans-serif", 30).into_font())
                .margin(10)
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(min_e as f32..max_e as f32, min_r..max_r)?;

            chart.configure_mesh()
                .x_desc("Episode")
                .y_desc("Reward")
                .draw()?;

            chart.draw_series(LineSeries::new(
                episodes.iter().zip(rewards.iter()).map(|(e, r)| (*e as f32, *r)),
                &RED,
            ))?;
        }

        Ok(output_path)
    }

    /// Generate score plot from training history
    pub fn plot_scores(&self, history: &TrainingHistory) -> Result<PathBuf> {
        let output_path = self.output_dir.join("scores.png");
        
        {
            let root = BitMapBackend::new(&output_path, (800, 600)).into_drawing_area();
            root.fill(&WHITE)?;

            let episodes: Vec<usize> = history.metrics.iter().map(|m| m.episode).collect();
            let scores: Vec<i32> = history.metrics.iter().map(|m| m.score).collect();
            
            let min_e = episodes.iter().min().copied().unwrap_or(0);
            let max_e = episodes.iter().max().copied().unwrap_or(100);
            let min_s = scores.iter().min().copied().unwrap_or(0);
            let max_s = scores.iter().max().copied().unwrap_or(1);

            let mut chart = ChartBuilder::on(&root)
                .caption("Training Scores", ("sans-serif", 30).into_font())
                .margin(10)
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(min_e as f32..max_e as f32, min_s as f32..max_s as f32)?;

            chart.configure_mesh()
                .x_desc("Episode")
                .y_desc("Score")
                .draw()?;

            chart.draw_series(LineSeries::new(
                episodes.iter().zip(scores.iter()).map(|(e, s)| (*e as f32, *s as f32)),
                &BLUE,
            ))?;
        }

        Ok(output_path)
    }

    /// Generate epsilon plot from training history
    pub fn plot_epsilon(&self, history: &TrainingHistory) -> Result<PathBuf> {
        let output_path = self.output_dir.join("epsilon.png");
        
        {
            let root = BitMapBackend::new(&output_path, (800, 600)).into_drawing_area();
            root.fill(&WHITE)?;

            let episodes: Vec<usize> = history.metrics.iter().map(|m| m.episode).collect();
            let epsilons: Vec<f64> = history.metrics.iter().map(|m| m.epsilon).collect();
            
            let min_e = episodes.iter().min().copied().unwrap_or(0);
            let max_e = episodes.iter().max().copied().unwrap_or(100);
            let min_eps = 0.0; // Epsilon is always positive
            let max_eps = epsilons.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).copied().unwrap_or(1.0);

            let mut chart = ChartBuilder::on(&root)
                .caption("Epsilon Decay", ("sans-serif", 30).into_font())
                .margin(10)
                .x_label_area_size(30)
                .y_label_area_size(30)
                .build_cartesian_2d(min_e as f32..max_e as f32, min_eps..max_eps)?;

            chart.configure_mesh()
                .x_desc("Episode")
                .y_desc("Epsilon")
                .draw()?;

            chart.draw_series(LineSeries::new(
                episodes.iter().zip(epsilons.iter()).map(|(e, eps)| (*e as f32, *eps)),
                &GREEN,
            ))?;
        }

        Ok(output_path)
    }

    /// Generate a summary HTML report with all plots
    pub fn generate_report(&self, history: &TrainingHistory) -> Result<PathBuf> {
        let rewards_path = self.plot_rewards(history)?;
        let scores_path = self.plot_scores(history)?;
        let epsilon_path = self.plot_epsilon(history)?;
        
        let report_path = self.output_dir.join("training_report.html");
        let mut file = File::create(&report_path)?;
        
        write!(file, r#"<!DOCTYPE html>
<html>
<head>
    <title>Training Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        h1 {{ color: #333; }}
        .plot-container {{ margin-bottom: 30px; }}
        .metrics {{ display: flex; justify-content: space-around; margin: 20px 0; }}
        .metric {{ text-align: center; padding: 10px; background-color: #f5f5f5; border-radius: 5px; }}
        .value {{ font-size: 24px; font-weight: bold; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ padding: 8px; text-align: left; border-bottom: 1px solid #ddd; }}
        th {{ background-color: #f2f2f2; }}
    </style>
</head>
<body>
    <h1>Reinforcement Learning Training Report</h1>
    
    <div class="metrics">
        <div class="metric">
            <div>Episodes</div>
            <div class="value">{}</div>
        </div>
        <div class="metric">
            <div>Best Score</div>
            <div class="value">{}</div>
        </div>
        <div class="metric">
            <div>Average Reward</div>
            <div class="value">{:.2}</div>
        </div>
    </div>
    
    <div class="plot-container">
        <h2>Reward Progress</h2>
        <img src="{}" alt="Rewards Plot" style="width: 100%;">
    </div>
    
    <div class="plot-container">
        <h2>Score Progress</h2>
        <img src="{}" alt="Scores Plot" style="width: 100%;">
    </div>
    
    <div class="plot-container">
        <h2>Epsilon Decay</h2>
        <img src="{}" alt="Epsilon Plot" style="width: 100%;">
    </div>
    
    <h2>Configuration</h2>
    <table>
        <tr>
            <th>Parameter</th>
            <th>Value</th>
        </tr>
        <tr>
            <td>Learning Rate</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Discount Factor</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Initial Epsilon</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Epsilon Decay</td>
            <td>{}</td>
        </tr>
        <tr>
            <td>Minimum Epsilon</td>
            <td>{}</td>
        </tr>
    </table>
</body>
</html>"#, 
            history.metrics.len(),
            history.metrics.iter().map(|m| m.score).max().unwrap_or(0),
            history.metrics.iter().map(|m| m.reward).sum::<f64>() / history.metrics.len() as f64,
            rewards_path.file_name().unwrap().to_string_lossy(),
            scores_path.file_name().unwrap().to_string_lossy(),
            epsilon_path.file_name().unwrap().to_string_lossy(),
            history.config.learning_rate,
            history.config.discount_factor,
            history.config.epsilon,
            history.config.epsilon_decay,
            history.config.min_epsilon,
        )?;
        
        Ok(report_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::rl::model::config::{TrainingConfig, TrainingMetrics};
    use tempfile::TempDir;

    #[test]
    fn test_visualization_tools() {
        let temp_dir = TempDir::new().unwrap();
        let viz = VisualizationTools::new(temp_dir.path());
        
        // Create a sample training history
        let config = TrainingConfig::default();
        let mut history = TrainingHistory::new(config);
        
        // Add some sample metrics
        for i in 0..100 {
            let metrics = TrainingMetrics {
                episode: i,
                reward: i as f64 * 0.1,
                score: i as i32 / 10,
                steps: i * 5,
                epsilon: 0.1 * (0.99_f64.powi(i as i32)),
                avg_q_value: Some(i as f64 * 0.05),
            };
            history.add_metrics(metrics);
        }
        
        // Just verify that the plots can be generated without errors
        let rewards_path = viz.plot_rewards(&history).unwrap();
        assert!(rewards_path.exists());
        
        let scores_path = viz.plot_scores(&history).unwrap();
        assert!(scores_path.exists());
        
        let epsilon_path = viz.plot_epsilon(&history).unwrap();
        assert!(epsilon_path.exists());
        
        let report_path = viz.generate_report(&history).unwrap();
        assert!(report_path.exists());
    }
} 
