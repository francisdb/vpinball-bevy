# vpinball-bevy

Seeing how far I get implementing Visual Pinball in rust/bevy

## Development

Make sure you have cargo installed. You can install it from [rustup.rs](https://rustup.rs/).

### Run

```bash
WGPU_BACKEND=vulkan cargo run -- "path/to/table.vpx"
```

For the WGPU backend issue see https://github.com/bevyengine/bevy/issues/14213
