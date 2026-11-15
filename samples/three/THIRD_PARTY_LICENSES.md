# Third-party licenses

`samples/three` and `samples/three-render` port code from three.js 0.187.0
(https://github.com/mrdoob/three.js) under the MIT license:

- `src/math/`, including `Color.js` and `ColorManagement.js`;
- `src/core/Object3D.js` and `src/core/BufferGeometry.js`;
- `src/cameras/`;
- `src/geometries/BoxGeometry.js` and `SphereGeometry.js`;
- `src/objects/Mesh.js`, `src/scenes/Scene.js` and
  `src/materials/MeshBasicMaterial.js`;
- `src/renderers/common/` and `src/renderers/webgpu/`, as far as
  `samples/three-render` uses them;
- `examples/jsm/controls/OrbitControls.js`.

`samples/three-render/src/wgsl` holds shaders that three.js's node builder
generated. The license is:

```
The MIT License

Copyright © 2010-2026 three.js authors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```
