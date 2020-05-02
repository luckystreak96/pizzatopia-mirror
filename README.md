# pizzatopia-amethyst

## Controls

**Playing**
- `Arrow keys` : Move and jump
- `Left Ctrl` : Change to editor mode

#### Editor - All modes controls
- `Left Ctrl` : Change back to playing
- `Arrow keys` : Move cursor
- `Insert` : Save your changes
####_Edit Mode_ controls (pink cursor mode):
- `X` : Pick up a block (arrows to move it around + X again to put it down)
- `Z` : Remove a block
- `A` : Enter _Insert Mode_
####_Insert Mode_ controls (grass cursor mode):
- `X` : Place a new block
- `1` : Change selected block to grass
- `2` : Change selected block to cat (the cat is invisible due to current bug, but it's there)
- `Z` : Return to _Edit Mode_


## How to run

To run the game, use

```
cargo run --features "vulkan"
```

on Windows and Linux, and

```
cargo run --features "metal"
```

on macOS.

For building without any graphics backend, you can use

```
cargo run --features "empty"
```

but be aware that as soon as you need any rendering you won't be able to run your game when using
the `empty` feature.
