## winit
This example spins up a blank winit-Window and prints frame time information.

## shader compilation
This example will compile a simple vertex shader from "assets/shader_compilation/original/example.vert".
The example vertex shader features an #include directive for a vertex definition in an adjacent file "example.glsl".
All paths have to be relative to the calling .exe. In a real application, you may use some kind of asset manager.

## egui
A more complex example around a custom egui renderer and graphics abstraction over vku.
It will render an interactive egui interface with a frame time information window and a cute 🦀.