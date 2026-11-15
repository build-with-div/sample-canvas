// The fewest flags with which Chrome for Testing 147.0.7727.50 gives WebGPU
// a SwiftShader adapter whose canvas textures can be read back (probe.mjs,
// docs/webgpu-oracle.md). Windows picks the hardware GPU without
// `--use-webgpu-adapter`, and `--use-angle=vulkan` leaves it no adapter.
// Linux destroys the device once a canvas is configured unless the
// compositor runs on Vulkan.
export const SWIFTSHADER = {
  win32: ['--enable-unsafe-webgpu', '--use-webgpu-adapter=swiftshader'],
  linux: ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan'],
}[process.platform];

export function swiftshaderFlags() {
  if (!SWIFTSHADER) throw new Error(`no SwiftShader flags measured for ${process.platform}`);
  return SWIFTSHADER;
}
