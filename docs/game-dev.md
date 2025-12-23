# Game Development with framealloc

Patterns and best practices for game engine development with framealloc.

## Table of Contents

1. [Game Loop Integration](#game-loop-integration)
2. [Entity Component Systems](#entity-component-systems)
3. [Rendering Pipeline](#rendering-pipeline)
4. [Audio System](#audio-system)
5. [Resource Management](#resource-management)
6. [Level Streaming](#level-streaming)
7. [Save/Load Systems](#save-load-systems)

## Game Loop Integration

### Basic Game Loop

```rust
use framealloc::SmartAlloc;

struct Game {
    alloc: SmartAlloc,
    renderer: Renderer,
    physics: PhysicsWorld,
    input: InputManager,
}

impl Game {
    fn new() -> Self {
        Self {
            alloc: SmartAlloc::new(Default::default()),
            renderer: Renderer::new(),
            physics: PhysicsWorld::new(),
            input: InputManager::new(),
        }
    }
    
    fn run(&mut self) {
        loop {
            // Begin frame - all temporary allocations
            self.alloc.begin_frame();
            
            // Input
            self.input.update();
            
            // Update
            self.update();
            
            // Render
            self.render();
            
            // End frame - everything freed
            self.alloc.end_frame();
            
            if self.input.should_quit() {
                break;
            }
        }
    }
    
    fn update(&mut self) {
        // Physics
        let events = self.physics.step_with_events(&self.alloc);
        self.process_physics_events(events);
        
        // Game logic
        self.update_game_logic();
        
        // Audio
        self.update_audio();
    }
}
```

### Variable Frame Rate

```rust
struct VariableStepGame {
    alloc: SmartAlloc,
    accumulator: f32,
    fixed_dt: f32,
}

impl VariableStepGame {
    fn update(&mut self, dt: f32) {
        self.accumulator += dt;
        
        // Fixed updates for physics
        while self.accumulator >= self.fixed_dt {
            self.fixed_update(self.fixed_dt);
            self.accumulator -= self.fixed_dt;
        }
        
        // Variable update for rendering
        let alpha = self.accumulator / self.fixed_dt;
        self.variable_update(alpha);
    }
    
    fn fixed_update(&mut self, dt: f32) {
        // Physics and game logic
        self.physics.step(dt);
    }
    
    fn variable_update(&mut self, alpha: f32) {
        // Interpolation for smooth rendering
        self.render_with_interpolation(alpha);
    }
}
```

## Entity Component Systems

### Component Storage

```rust
use framealloc::SmartAlloc;
use std::collections::HashMap;

struct ComponentStorage<T> {
    components: HashMap<EntityId, T>,
    frame_snapshot: Option<FrameBox<HashMap<EntityId, T>>>,
}

impl<T: Clone> ComponentStorage<T> {
    fn new() -> Self {
        Self {
            components: HashMap::new(),
            frame_snapshot: None,
        }
    }
    
    fn get_frame_snapshot(&mut self, alloc: &SmartAlloc) -> &HashMap<EntityId, T> {
        if self.frame_snapshot.is_none() {
            self.frame_snapshot = Some(alloc.frame_box(self.components.clone()));
        }
        self.frame_snapshot.as_ref().unwrap()
    }
    
    fn add(&mut self, entity: EntityId, component: T) {
        self.components.insert(entity, component);
    }
    
    fn remove(&mut self, entity: EntityId) -> Option<T> {
        self.components.remove(&entity)
    }
}
```

### System Implementation

```rust
struct MovementSystem {
    velocity_storage: ComponentStorage<Velocity>,
    position_storage: ComponentStorage<Position>,
}

impl MovementSystem {
    fn update(&mut self, alloc: &SmartAlloc, dt: f32) {
        alloc.begin_frame();
        
        // Get frame snapshot for consistent iteration
        let velocities = self.velocity_storage.get_frame_snapshot(alloc);
        let positions = &mut self.position_storage.components;
        
        // Update positions
        for (entity, velocity) in velocities.iter() {
            if let Some(position) = positions.get_mut(entity) {
                position.x += velocity.x * dt;
                position.y += velocity.y * dt;
                position.z += velocity.z * dt;
            }
        }
        
        alloc.end_frame();
    }
}
```

### Query System

```rust
struct Query<'a, T> {
    entities: FrameBox<Vec<(EntityId, &'a T)>>,
}

impl<'a, T> Query<'a, T> {
    fn new(storage: &'a ComponentStorage<T>, alloc: &SmartAlloc) -> Self {
        alloc.begin_frame();
        
        let entities = alloc.frame_box(
            storage.components.iter()
                .map(|(id, comp)| (*id, comp))
                .collect()
        );
        
        alloc.end_frame();
        Self { entities }
    }
    
    fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities.iter().map(|(id, comp)| (*id, *comp))
    }
}

// Usage
fn query_positions(positions: &ComponentStorage<Position>, alloc: &SmartAlloc) {
    let query = Query::new(positions, alloc);
    for (entity, position) in query.iter() {
        println!("Entity {:?} at {:?}", entity, position);
    }
}
```

## Rendering Pipeline

### Command Buffer

```rust
use framealloc::SmartAlloc;

enum RenderCommand {
    Clear(Color),
    DrawMesh(MeshHandle, Transform),
    DrawText(String, Position),
    SetCamera(Camera),
}

struct RenderFrame {
    commands: FrameBox<Vec<RenderCommand>>,
    uniform_buffers: HashMap<String, FrameBox<UniformBuffer>>,
    vertex_buffers: Vec<FrameBox<VertexBuffer>>,
}

impl RenderFrame {
    fn new(alloc: &SmartAlloc) -> Self {
        alloc.begin_frame();
        
        Self {
            commands: alloc.frame_box(Vec::new()),
            uniform_buffers: HashMap::new(),
            vertex_buffers: Vec::new(),
        }
    }
    
    fn add_command(&mut self, command: RenderCommand) {
        self.commands.push(command);
    }
    
    fn get_uniform_buffer(&mut self, alloc: &SmartAlloc, name: &str, size: usize) -> &mut UniformBuffer {
        if !self.uniform_buffers.contains_key(name) {
            let buffer = alloc.frame_box(UniformBuffer::new(size));
            self.uniform_buffers.insert(name.to_string(), buffer);
        }
        self.uniform_buffers.get_mut(name).unwrap()
    }
}

impl Drop for RenderFrame {
    fn drop(&mut self) {
        // All GPU resources automatically returned to pool
    }
}
```

### Batching System

```rust
struct RenderBatch {
    material: MaterialHandle,
    mesh: MeshHandle,
    instances: FrameBox<Vec<InstanceData>>,
}

struct BatchingRenderer {
    batches: HashMap<(MaterialHandle, MeshHandle), RenderBatch>,
}

impl BatchingRenderer {
    fn add_instance(&mut self, alloc: &SmartAlloc, material: MaterialHandle, mesh: MeshHandle, transform: Transform) {
        let key = (material, mesh);
        let batch = self.batches.entry(key).or_insert_with(|| RenderBatch {
            material,
            mesh,
            instances: alloc.frame_box(Vec::new()),
        });
        
        batch.instances.push(InstanceData::from_transform(transform));
    }
    
    fn flush(&mut self, renderer: &mut Renderer) {
        for batch in self.batches.values() {
            renderer.draw_instanced(batch.mesh, batch.material, &batch.instances);
        }
    }
}
```

### Culling System

```rust
struct CullingSystem {
    camera: Camera,
    visible_entities: FrameBox<Vec<EntityId>>,
}

impl CullingSystem {
    fn cull_entities(&mut self, alloc: &SmartAlloc, entities: &[Entity], transforms: &ComponentStorage<Transform>) {
        alloc.begin_frame();
        
        self.visible_entities = alloc.frame_box(
            entities.iter()
                .filter(|entity| {
                    if let Some(transform) = transforms.components.get(entity) {
                        self.camera.is_in_view(transform.position, transform.bounds)
                    } else {
                        false
                    }
                })
                .copied()
                .collect()
        );
        
        alloc.end_frame();
    }
}
```

## Audio System

### Audio Mixer

```rust
use framealloc::SmartAlloc;

struct AudioFrame {
    samples: FrameBox<[f32]>,
    voices: Vec<Voice>,
}

struct Voice {
    buffer: PoolBox<AudioBuffer>,
    position: usize,
    volume: f32,
    pitch: f32,
}

impl AudioFrame {
    fn new(sample_rate: usize, alloc: &SmartAlloc) -> Self {
        alloc.begin_frame();
        
        Self {
            samples: alloc.frame_slice::<f32>(sample_rate / 60), // 60 FPS
            voices: Vec::new(),
        }
    }
    
    fn mix(&mut self) {
        // Clear buffer
        for sample in self.samples.iter_mut() {
            *sample = 0.0;
        }
        
        // Mix all voices
        for voice in &self.voices {
            for (i, sample) in self.samples.iter_mut().enumerate() {
                if voice.position + i < voice.buffer.len() {
                    *sample += voice.buffer[voice.position + i] * voice.volume;
                }
            }
        }
    }
}
```

### 3D Audio

```rust
struct Audio3D {
    listener: Transform,
    voices: Vec<Voice3D>,
}

struct Voice3D {
    voice: Voice,
    position: Vector3,
    radius: f32,
}

impl Audio3D {
    fn calculate_gain(&self, voice: &Voice3D) -> f32 {
        let distance = (voice.position - self.listener.position).magnitude();
        if distance < voice.radius {
            1.0 - (distance / voice.radius)
        } else {
            0.0
        }
    }
    
    fn mix_frame(&mut self, alloc: &SmartAlloc) -> AudioFrame {
        let mut frame = AudioFrame::new(44100 / 60, alloc);
        
        for voice in &mut self.voices {
            let gain = self.calculate_gain(voice);
            voice.voice.volume = gain;
            frame.voices.push(voice.voice.clone());
        }
        
        frame.mix();
        frame
    }
}
```

## Resource Management

### Asset Loading

```rust
struct AssetLoader {
    loading_queue: Vec<LoadRequest>,
    loaded_assets: HashMap<String, AssetHandle>,
    frame_assets: FrameBox<HashMap<String, AssetHandle>>,
}

impl AssetLoader {
    fn update(&mut self, alloc: &SmartAlloc) {
        alloc.begin_frame();
        
        // Process loading queue
        for request in self.loading_queue.drain(..) {
            if let Some(asset) = self.load_asset(&request.path) {
                self.loaded_assets.insert(request.path.clone(), asset);
            }
        }
        
        // Create frame snapshot
        self.frame_assets = alloc.frame_box(self.loaded_assets.clone());
        
        alloc.end_frame();
    }
    
    fn get_asset(&self, path: &str) -> Option<AssetHandle> {
        self.frame_assets.get(path).copied()
    }
}
```

### Texture Streaming

```rust
struct TextureStreamer {
    textures: HashMap<String, Texture>,
    mip_levels: usize,
    stream_radius: f32,
}

impl TextureStreamer {
    fn update(&mut self, alloc: &SmartAlloc, camera_position: Vector3) {
        alloc.begin_frame();
        
        // Stream in nearby textures
        for (path, texture) in &mut self.textures {
            let distance = (texture.position - camera_position).magnitude();
            
            if distance < self.stream_radius {
                // Stream higher mip levels
                let target_mip = self.calculate_target_mip(distance);
                while texture.loaded_mip < target_mip {
                    if let Some(mip_data) = self.load_mip_level(path, texture.loaded_mip + 1) {
                        texture.upload_mip(texture.loaded_mip + 1, &mip_data);
                        texture.loaded_mip += 1;
                    } else {
                        break;
                    }
                }
            } else {
                // Stream out lower mip levels
                let target_mip = self.calculate_target_mip(distance);
                while texture.loaded_mip > target_mip {
                    texture.unload_mip(texture.loaded_mip);
                    texture.loaded_mip -= 1;
                }
            }
        }
        
        alloc.end_frame();
    }
}
```

### Memory Budgeting

```rust
struct ResourceBudget {
    textures: usize,
    meshes: usize,
    audio: usize,
    total: usize,
}

impl ResourceBudget {
    fn check_budget(&self, current_usage: &ResourceUsage) -> bool {
        current_usage.textures <= self.textures &&
        current_usage.meshes <= self.meshes &&
        current_usage.audio <= self.audio &&
        current_usage.total <= self.total
    }
    
    fn evict_if_needed(&self, usage: &mut ResourceUsage, asset_type: AssetType) {
        if !self.check_budget(usage) {
            match asset_type {
                AssetType::Texture => self.evict_textures(usage),
                AssetType::Mesh => self.evict_meshes(usage),
                AssetType::Audio => self.evict_audio(usage),
            }
        }
    }
}
```

## Level Streaming

### World Grid

```rust
struct WorldGrid {
    chunks: HashMap<ChunkCoord, Chunk>,
    load_radius: usize,
    unload_radius: usize,
}

#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct ChunkCoord {
    x: i32,
    y: i32,
    z: i32,
}

struct Chunk {
    coord: ChunkCoord,
    entities: Vec<Entity>,
    loaded: bool,
}

impl WorldGrid {
    fn update(&mut self, alloc: &SmartAlloc, player_position: Vector3) {
        alloc.begin_frame();
        
        let player_chunk = self.world_to_chunk(player_position);
        
        // Load nearby chunks
        for x in (player_chunk.x - self.load_radius as i32)..=(player_chunk.x + self.load_radius as i32) {
            for y in (player_chunk.y - self.load_radius as i32)..=(player_chunk.y + self.load_radius as i32) {
                for z in (player_chunk.z - self.load_radius as i32)..=(player_chunk.z + self.load_radius as i32) {
                    let coord = ChunkCoord { x, y, z };
                    if !self.chunks.contains_key(&coord) {
                        self.load_chunk(coord, alloc);
                    }
                }
            }
        }
        
        // Unload distant chunks
        self.chunks.retain(|coord, chunk| {
            let distance = (coord.x - player_chunk.x).abs() as usize +
                         (coord.y - player_chunk.y).abs() as usize +
                         (coord.z - player_chunk.z).abs() as usize;
            distance <= self.unload_radius
        });
        
        alloc.end_frame();
    }
}
```

### Streaming System

```rust
struct StreamingSystem {
    world: WorldGrid,
    loading_queue: VecDeque<ChunkLoadRequest>,
    streaming_thread: JoinHandle<()>,
}

impl StreamingSystem {
    fn update(&mut self, alloc: &SmartAlloc, player_position: Vector3) {
        // Update world grid
        self.world.update(alloc, player_position);
        
        // Process loaded chunks from streaming thread
        while let Some(loaded_chunk) = self.try_receive_loaded_chunk() {
            self.world.insert_chunk(loaded_chunk);
        }
    }
    
    fn load_chunk(&mut self, coord: ChunkCoord, alloc: &SmartAlloc) {
        let request = ChunkLoadRequest { coord };
        self.loading_queue.push_back(request);
    }
}
```

## Save/Load Systems

### Serialization

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SaveData {
    version: u32,
    player_position: Vector3,
    inventory: Vec<Item>,
    quest_progress: HashMap<String, u32>,
    world_state: WorldState,
}

impl SaveData {
    fn capture(game: &Game, alloc: &SmartAlloc) -> FrameBox<SaveData> {
        alloc.begin_frame();
        
        let data = SaveData {
            version: 1,
            player_position: game.player.position,
            inventory: game.player.inventory.clone(),
            quest_progress: game.quests.progress.clone(),
            world_state: game.world.capture_state(),
        };
        
        alloc.end_frame();
        alloc.frame_box(data)
    }
    
    fn save_to_file(&self, path: &str) -> Result<(), Error> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
```

### Checkpoint System

```rust
struct CheckpointSystem {
    checkpoints: Vec<Checkpoint>,
    current_checkpoint: usize,
    auto_save_interval: f32,
    time_since_save: f32,
}

struct Checkpoint {
    timestamp: f32,
    save_data: PoolBox<SaveData>,
    screenshot: PoolBox<Image>,
}

impl CheckpointSystem {
    fn update(&mut self, game: &Game, dt: f32, alloc: &SmartAlloc) {
        self.time_since_save += dt;
        
        if self.time_since_save >= self.auto_save_interval {
            self.create_checkpoint(game, alloc);
            self.time_since_save = 0.0;
        }
    }
    
    fn create_checkpoint(&mut self, game: &Game, alloc: &SmartAlloc) {
        alloc.begin_frame();
        
        let save_data = SaveData::capture(game, alloc);
        let screenshot = self.capture_screenshot();
        
        alloc.end_frame();
        
        let checkpoint = Checkpoint {
            timestamp: game.time,
            save_data: alloc.pool_box(save_data.into_inner()),
            screenshot: alloc.pool_box(screenshot),
        };
        
        self.checkpoints.push(checkpoint);
        
        // Keep only last 10 checkpoints
        if self.checkpoints.len() > 10 {
            self.checkpoints.remove(0);
        }
    }
}
```

## Performance Patterns

### Frame Rate Independence

```rust
struct FrameRateIndependentSystem {
    accumulator: f32,
    fixed_dt: f32,
    max_steps: usize,
}

impl FrameRateIndependentSystem {
    fn update(&mut self, dt: f32) {
        self.accumulator += dt;
        
        // Limit steps to prevent spiral of death
        let steps = (self.accumulator / self.fixed_dt) as usize;
        let steps = steps.min(self.max_steps);
        
        for _ in 0..steps {
            self.fixed_update(self.fixed_dt);
            self.accumulator -= self.fixed_dt;
        }
    }
}
```

### Object Pooling

```rust
struct GameObjectPool<T> {
    available: Vec<PoolBox<T>>,
    active: Vec<PoolBox<T>>,
    max_size: usize,
}

impl<T: Default> GameObjectPool<T> {
    fn get(&mut self, alloc: &SmartAlloc) -> PoolBox<T> {
        if let Some(obj) = self.available.pop() {
            obj
        } else if self.active.len() < self.max_size {
            alloc.pool_box(T::default())
        } else {
            // Reuse oldest active object
            self.active.remove(0)
        }
    }
    
    fn return_object(&mut self, obj: PoolBox<T>) {
        self.available.push(obj);
    }
    
    fn update(&mut self) {
        // Update all active objects
        for obj in &mut self.active {
            // Update object
        }
    }
}
```

## Best Practices

### Memory Management

1. **Frame boundaries** - Always match begin_frame/end_frame
2. **Pool persistence** - Use pools for objects that survive frames
3. **Batch operations** - Group similar allocations
4. **Budgets** - Set limits to prevent memory bloat

### Performance

1. **Profile first** - Measure before optimizing
2. **Cache locality** - Keep related data together
3. **Avoid fragmentation** - Use appropriate allocation types
4. **Minimize allocations** - Reuse when possible

### Architecture

1. **Separate concerns** - Keep systems independent
2. **Data-oriented** - Structure for cache efficiency
3. **Frame awareness** - Design around frame boundaries
4. **Thread safety** - Use TransferHandle for sharing

## Further Reading

- [Getting Started](getting-started.md) - Basic concepts
- [Patterns Guide](patterns.md) - Common patterns
- [Performance Guide](performance.md) - Optimization
- [Rapier Integration](rapier-integration.md) - Physics

Happy game development! 🎮
