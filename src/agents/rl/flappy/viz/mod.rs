use pixels::{Pixels, SurfaceTexture};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;
use super::FlappyBirdState;

pub struct FlappyViz {
    pixels: Pixels,
    input: WinitInputHelper,
}

impl FlappyViz {
    pub fn new(window: &Window) -> Self {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
        let pixels = Pixels::new(288, 512, surface_texture).unwrap();
        let input = WinitInputHelper::new();

        Self { pixels, input }
    }

    pub fn render(&mut self, state: &FlappyBirdState) {
        let frame = self.pixels.frame_mut();
        
        // Clear screen (sky blue)
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 135; // R
            pixel[1] = 206; // G
            pixel[2] = 235; // B
            pixel[3] = 255; // A
        }

        // Draw bird (yellow circle)
        Self::draw_circle(
            state.bird_y as f32,
            144.0, // center of screen horizontally
            12.0,  // bird radius
            &[255, 255, 0, 255],
            frame,
        );

        // Draw pipes (green rectangles)
        Self::draw_pipe(
            state.next_pipe_dist as f32,
            0.0,
            state.next_pipe_top as f32,
            &[0, 255, 0, 255],
            frame,
        );
        Self::draw_pipe(
            state.next_pipe_dist as f32,
            state.next_pipe_bottom as f32,
            512.0,
            &[0, 255, 0, 255],
            frame,
        );

        // Draw score
        Self::draw_score(state.score, frame);

        self.pixels.render().unwrap();
    }

    fn draw_circle(y: f32, x: f32, radius: f32, color: &[u8; 4], frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let px = (i % 288) as f32;
            let py = (i / 288) as f32;
            
            let dx = px - x;
            let dy = py - y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance <= radius {
                pixel.copy_from_slice(color);
            }
        }
    }

    fn draw_pipe(x: f32, y1: f32, y2: f32, color: &[u8; 4], frame: &mut [u8]) {
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let px = (i % 288) as f32;
            let py = (i / 288) as f32;
            
            if px >= x && px <= x + 52.0 && py >= y1 && py <= y2 {
                pixel.copy_from_slice(color);
            }
        }
    }

    fn draw_score(score: i32, frame: &mut [u8]) {
        let score_str = score.to_string();
        let x = 10.0;
        let y = 10.0;
        
        for (i, c) in score_str.chars().enumerate() {
            Self::draw_digit(c, x + (i as f32 * 20.0), y, &[255, 255, 255, 255], frame);
        }
    }

    fn draw_digit(digit: char, x: f32, y: f32, color: &[u8; 4], frame: &mut [u8]) {
        let segments = match digit {
            '0' => vec![(0,0,1,2), (0,0,2,0), (2,0,2,2), (0,2,2,2)],
            '1' => vec![(1,0,1,2)],
            '2' => vec![(0,0,2,0), (2,0,2,1), (0,1,2,1), (0,1,0,2), (0,2,2,2)],
            '3' => vec![(0,0,2,0), (2,0,2,2), (0,1,2,1), (0,2,2,2)],
            '4' => vec![(0,0,0,1), (0,1,2,1), (2,0,2,2)],
            '5' => vec![(0,0,2,0), (0,0,0,1), (0,1,2,1), (2,1,2,2), (0,2,2,2)],
            '6' => vec![(0,0,2,0), (0,0,0,2), (0,1,2,1), (2,1,2,2), (0,2,2,2)],
            '7' => vec![(0,0,2,0), (2,0,2,2)],
            '8' => vec![(0,0,2,0), (0,0,0,2), (2,0,2,2), (0,1,2,1), (0,2,2,2)],
            '9' => vec![(0,0,2,0), (0,0,0,1), (2,0,2,2), (0,1,2,1), (0,2,2,2)],
            _ => vec![],
        };

        for (x1, y1, x2, y2) in segments {
            Self::draw_line(
                x + x1 as f32 * 5.0,
                y + y1 as f32 * 5.0,
                x + x2 as f32 * 5.0,
                y + y2 as f32 * 5.0,
                color,
                frame,
            );
        }
    }

    fn draw_line(x1: f32, y1: f32, x2: f32, y2: f32, color: &[u8; 4], frame: &mut [u8]) {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let steps = dx.abs().max(dy.abs()) as usize;

        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let x = x1 + dx * t;
            let y = y1 + dy * t;
            
            if x >= 0.0 && x < 288.0 && y >= 0.0 && y < 512.0 {
                let index = (y as usize * 288 + x as usize) * 4;
                if index + 3 < frame.len() {
                    frame[index..index + 4].copy_from_slice(color);
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(feature = "rl")]
mod tests {
    use super::*;
    use winit::window::WindowBuilder;
    use winit::event_loop::EventLoop;

    #[test]
    fn test_viz_creation() {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Flappy Bird")
            .with_inner_size(winit::dpi::LogicalSize::new(288.0, 512.0))
            .build(&event_loop)
            .unwrap();

        let _viz = FlappyViz::new(&window);
    }
} 
