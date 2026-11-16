import SwiftUI
import MetalKit

public struct NovaMetalViewport: UIViewRepresentable {
    @ObservedObject var bridge: NovaBridge

    public init(bridge: NovaBridge) {
        self.bridge = bridge
    }

    public func makeCoordinator() -> Coordinator {
        Coordinator(bridge: bridge)
    }

    public func makeUIView(context: Context) -> MTKView {
        let mtkView = MTKView()
        mtkView.device = MTLCreateSystemDefaultDevice()
        mtkView.colorPixelFormat = .bgra8Unorm
        mtkView.depthStencilPixelFormat = .depth32Float
        mtkView.clearColor = MTLClearColor(red: 0.051, green: 0.055, blue: 0.067, alpha: 1.0)
        mtkView.preferredFramesPerSecond = 120 // ProMotion 120Hz support on iPad Pro
        mtkView.delegate = context.coordinator

        // Multi-touch and Apple Pencil gesture configuration
        let panGesture = UIPanGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePan(_:)))
        panGesture.maximumNumberOfTouches = 2
        mtkView.addGestureRecognizer(panGesture)

        let pinchGesture = UIPinchGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handlePinch(_:)))
        mtkView.addGestureRecognizer(pinchGesture)

        let tapGesture = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.handleTap(_:)))
        mtkView.addGestureRecognizer(tapGesture)

        return mtkView
    }

    public func updateUIView(_ uiView: MTKView, context: Context) {
        // Dynamic view updates
    }

    @MainActor
    public class Coordinator: NSObject, MTKViewDelegate {
        var bridge: NovaBridge
        private var lastPanPoint: CGPoint = .zero
        private var isGizmoDragging: Bool = false

        init(bridge: NovaBridge) {
            self.bridge = bridge
            super.init()
        }

        nonisolated public func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
            // Viewport resize handling
        }

        nonisolated public func draw(in view: MTKView) {
            guard let drawable = view.currentDrawable,
                  let renderPassDescriptor = view.currentRenderPassDescriptor,
                  let commandQueue = view.device?.makeCommandQueue(),
                  let commandBuffer = commandQueue.makeCommandBuffer() else {
                return
            }

            // Perform render pass
            if let encoder = commandBuffer.makeRenderCommandEncoder(descriptor: renderPassDescriptor) {
                encoder.endEncoding()
            }

            commandBuffer.present(drawable)
            commandBuffer.commit()
        }

        @objc func handleTap(_ gesture: UITapGestureRecognizer) {
            guard let view = gesture.view else { return }
            let loc = gesture.location(in: view)
            let w = Float(view.bounds.width)
            let h = Float(view.bounds.height)
            let aspect = w / max(h, 1.0)
            let ndcX = (2.0 * Float(loc.x) / w) - 1.0
            let ndcY = 1.0 - (2.0 * Float(loc.y) / h)

            bridge.pickNode(ndcX: ndcX, ndcY: ndcY, aspect: aspect)
        }

        @objc func handlePan(_ gesture: UIPanGestureRecognizer) {
            guard let view = gesture.view else { return }
            let loc = gesture.location(in: view)
            let w = Float(view.bounds.width)
            let h = Float(view.bounds.height)
            let aspect = w / max(h, 1.0)
            let ndcX = (2.0 * Float(loc.x) / w) - 1.0
            let ndcY = 1.0 - (2.0 * Float(loc.y) / h)

            switch gesture.state {
            case .began:
                lastPanPoint = loc
                let hitAxis = bridge.pickGizmoAxis(ndcX: ndcX, ndcY: ndcY, aspect: aspect)
                if hitAxis != 0 {
                    isGizmoDragging = bridge.beginGizmoDrag(axis: hitAxis, ndcX: ndcX, ndcY: ndcY, aspect: aspect)
                } else {
                    isGizmoDragging = false
                }
            case .changed:
                if isGizmoDragging {
                    _ = bridge.updateGizmoDrag(ndcX: ndcX, ndcY: ndcY, aspect: aspect)
                } else {
                    let touches = gesture.numberOfTouches
                    let deltaX = Float(loc.x - lastPanPoint.x)
                    let deltaY = Float(loc.y - lastPanPoint.y)

                    if touches >= 2 {
                        // Two finger pan
                        bridge.pan(dx: deltaX / h, dy: deltaY / h)
                    } else {
                        // Single finger orbit
                        let orbitSpeed: Float = 3.2 / h
                        bridge.orbit(deltaYaw: -deltaX * orbitSpeed, deltaPitch: deltaY * orbitSpeed)
                    }
                }
                lastPanPoint = loc
            case .ended, .cancelled:
                if isGizmoDragging {
                    bridge.endGizmoDrag()
                    isGizmoDragging = false
                }
            default:
                break
            }
        }

        @objc func handlePinch(_ gesture: UIPinchGestureRecognizer) {
            if gesture.state == .changed {
                let zoomFactor = Float(1.0 / gesture.scale)
                bridge.zoom(factor: zoomFactor)
                gesture.scale = 1.0
            }
        }
    }
}
