//! Frame loop example
//! 
//! Demonstrates typical game loop with frame allocations

use framealloc::SmartAlloc;
use std::time::{Duration, Instant};

struct GameState {
    player_x: f32,
    player_y: f32,
    enemies: Vec<(f32, f32)>,
    score: u32,
}

impl GameState {
    fn new() -> Self {
        Self {
            player_x: 0.0,
            player_y: 0.0,
            enemies: vec![(10.0, 0.0), (-10.0, 5.0), (0.0, -10.0)],
            score: 0,
        }
    }
    
    fn update(&mut self, dt: f32) {
        // Move player
        self.player_x += dt * 5.0;
        
        // Move enemies
        for (x, y) in &mut self.enemies {
            *y -= dt * 3.0;
        }
        
        // Check collisions
        self.enemies.retain(|&(x, y)| {
            let dist = ((x - self.player_x).powi(2) + (y - self.player_y).powi(2)).sqrt();
            if dist < 2.0 {
                self.score += 10;
                false // Remove enemy
            } else {
                true // Keep enemy
            }
        });
    }
}

fn main() {
    let alloc = SmartAlloc::new(Default::default());
    let mut game_state = GameState::new();
    
    println!("Starting game loop...");
    
    let mut frame_count = 0;
    let start_time = Instant::now();
    let target_fps = 60;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    
    loop {
        let frame_start = Instant::now();
        
        // Begin frame - all temporary allocations
        alloc.begin_frame();
        
        // Input processing (simulated)
        let input_events = alloc.frame_vec::<InputEvent>();
        if frame_count % 60 == 0 {
            input_events.push(InputEvent::Jump);
        }
        
        // Update game logic
        let dt = frame_duration.as_secs_f32();
        game_state.update(dt);
        
        // Process input events
        for event in &input_events {
            match event {
                InputEvent::Jump => game_state.player_y += 5.0,
            }
        }
        
        // Render frame (simulated)
        render_frame(&alloc, &game_state, frame_count);
        
        // End frame - everything freed
        alloc.end_frame();
        
        frame_count += 1;
        
        // Print stats every 60 frames
        if frame_count % 60 == 0 {
            let elapsed = start_time.elapsed();
            let fps = frame_count as f64 / elapsed.as_secs_f64();
            println!("Frame: {}, FPS: {:.1}, Score: {}", frame_count, fps, game_state.score);
        }
        
        // Stop after 5 seconds
        if start_time.elapsed() >= Duration::from_secs(5) {
            break;
        }
        
        // Frame rate limiting
        let frame_time = frame_start.elapsed();
        if frame_time < frame_duration {
            std::thread::sleep(frame_duration - frame_time);
        }
    }
    
    println!("\nGame over! Final score: {}", game_state.score);
    println!("Total frames: {}", frame_count);
}

#[derive(Debug)]
enum InputEvent {
    Jump,
    Shoot,
    Move(f32, f32),
}

fn render_frame(alloc: &SmartAlloc, state: &GameState, frame: usize) {
    // Allocate render commands
    let commands = alloc.frame_vec::<RenderCommand>();
    
    // Add player
    commands.push(RenderCommand::DrawCircle {
        x: state.player_x,
        y: state.player_y,
        radius: 1.0,
        color: 0x00FF00,
    });
    
    // Add enemies
    for &(x, y) in &state.enemies {
        commands.push(RenderCommand::DrawRect {
            x: x - 0.5,
            y: y - 0.5,
            w: 1.0,
            h: 1.0,
            color: 0xFF0000,
        });
    }
    
    // Add score
    commands.push(RenderCommand::DrawText {
        x: -10.0,
        y: 9.0,
        text: format!("Score: {}", state.score),
        color: 0xFFFFFF,
    });
    
    // Simulate rendering
    if frame % 30 == 0 {
        println!("Rendering {} commands", commands.len());
    }
}

#[derive(Debug)]
enum RenderCommand {
    DrawCircle { x: f32, y: f32, radius: f32, color: u32 },
    DrawRect { x: f32, y: f32, w: f32, h: f32, color: u32 },
    DrawText { x: f32, y: f32, text: String, color: u32 },
}
