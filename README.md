# 7th Sense Frame Viewer

A small Rust viewer for macOS that opens individual .7th files and steps through numbered frame sequences. It recognises the tested 28-byte header and 8-bit packed YUYV 4:2:2 layout. The window displays a preview up to 2048 pixels on the longest edge.

## Run

With Rust and the Xcode Command Line Tools installed, run:

    cargo run --release -- path/to/frame.7th

You can also start the viewer without arguments and drag a frame into its window. Command-O opens the file picker. Left and right arrow keys step through files in the same directory with the same prefix and frame-number padding.

The default preview uses full-range BT.709 and displays the first stored row at the top. Controls allow BT.601, video range and vertical flipping.

## Export a PNG preview

    cargo run --release -- --preview frame.7th preview.png

Choose a new output filename. Run the decoder tests with `cargo test --locked`.
