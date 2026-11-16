import Foundation
import Combine
import SwiftUI

public enum GizmoMode: Int32, CaseIterable {
    case select = 0
    case move = 1
    case rotate = 2
    case scale = 3
}

public enum InteractionMode: Int32 {
    case object = 0
    case edit = 1
}

public enum EditSelectMode: Int32, CaseIterable {
    case vertex = 0
    case edge = 1
    case face = 2
}

public enum ShadingMode: Int32, CaseIterable {
    case solid = 0
    case wireframe = 1
    case materialPreview = 2
    case renderPreview = 3
}

public enum StandardView: Int32, CaseIterable {
    case top = 0
    case bottom = 1
    case front = 2
    case back = 3
    case right = 4
    case left = 5
    case isometric = 6
}

public struct SceneNodeItem: Identifiable, Equatable {
    public let id: UInt64
    public let name: String
    public let kind: UInt32
    public let visible: Bool
    public let selected: Bool
    public let primarySelected: Bool
    public let hasMesh: Bool
}

public struct NodeDetails {
    public var id: UInt64 = 0
    public var name: String = ""
    public var position: (Float, Float, Float) = (0, 0, 0)
    public var rotation: (Float, Float, Float) = (0, 0, 0)
    public var scale: (Float, Float, Float) = (1, 1, 1)
    public var baseColor: (Float, Float, Float, Float) = (0.7, 0.7, 0.7, 1.0)
    public var metallic: Float = 0.0
    public var roughness: Float = 0.5
}

@MainActor
public final class NovaBridge: ObservableObject {
    @Published public private(set) var isReady: Bool = false
    @Published public private(set) var projectTitle: String = "Untitled Studio"
    @Published public private(set) var hasUnsavedChanges: Bool = false
    @Published public var interactionMode: InteractionMode = .object
    @Published public var editSelectMode: EditSelectMode = .face
    @Published public var gizmoMode: GizmoMode = .move
    @Published public var shadingMode: ShadingMode = .materialPreview
    @Published public private(set) var sceneNodes: [SceneNodeItem] = []
    @Published public private(set) var selectedNode: NodeDetails? = nil
    @Published public private(set) var vertexCount: Int = 0
    @Published public private(set) var edgeCount: Int = 0
    @Published public private(set) var faceCount: Int = 0
    @Published public private(set) var statusMessage: String? = nil

    private var handle: OpaquePointer? = nil
    private let queue = DispatchQueue(label: "com.nova3d.core", qos: .userInteractive)

    public init() {
        startSession()
    }

    deinit {
        if let h = handle {
            nova_core_destroy(h)
        }
    }

    public func startSession() {
        queue.async { [weak self] in
            let h = nova_core_create()
            nova_core_scene_reset_starter(h)
            DispatchQueue.main.async {
                self?.handle = h
                self?.isReady = true
                self?.refreshSceneState()
            }
        }
    }

    public func refreshSceneState() {
        guard let h = handle else { return }
        let count = nova_core_scene_node_count(h)
        var nodes: [SceneNodeItem] = []
        var selectedItem: NodeDetails? = nil

        for i in 0..<count {
            var meta = NovaNodeMeta()
            if nova_core_scene_get_node_meta(h, i, &meta) == 0 {
                var nameBuf = [CChar](repeating: 0, count: 64)
                _ = nova_core_scene_get_node_name(h, meta.id, &nameBuf, 64)
                let name = String(cString: nameBuf)

                let item = SceneNodeItem(
                    id: meta.id,
                    name: name.isEmpty ? "Node \(meta.id)" : name,
                    kind: meta.kind,
                    visible: meta.visible,
                    selected: meta.selected,
                    primarySelected: meta.primary_selected,
                    hasMesh: meta.has_mesh
                )
                nodes.append(item)

                if meta.primary_selected {
                    var details = NovaNodeDetails()
                    if nova_core_scene_get_node_details(h, meta.id, &details) == 0 {
                        selectedItem = NodeDetails(
                            id: details.id,
                            name: item.name,
                            position: (details.position.0, details.position.1, details.position.2),
                            rotation: (details.rotation_euler.0, details.rotation_euler.1, details.rotation_euler.2),
                            scale: (details.scale.0, details.scale.1, details.scale.2),
                            baseColor: (details.base_color.0, details.base_color.1, details.base_color.2, details.base_color.3),
                            metallic: details.metallic,
                            roughness: details.roughness
                        )
                    }
                }
            }
        }

        self.sceneNodes = nodes
        self.selectedNode = selectedItem

        var editState = NovaEditState()
        if nova_core_edit_get_state(h, &editState) == 0 {
            self.vertexCount = Int(editState.vertex_count)
            self.edgeCount = Int(editState.edge_count)
            self.faceCount = Int(editState.face_count)
        }
    }

    // MARK: - Camera Controls
    public func orbit(deltaYaw: Float, deltaPitch: Float) {
        guard let h = handle else { return }
        nova_core_camera_orbit(h, deltaYaw, deltaPitch)
    }

    public func pan(dx: Float, dy: Float) {
        guard let h = handle else { return }
        nova_core_camera_pan(h, dx, dy)
    }

    public func zoom(factor: Float) {
        guard let h = handle else { return }
        nova_core_camera_zoom(h, factor)
    }

    public func setStandardView(_ view: StandardView) {
        guard let h = handle else { return }
        nova_core_camera_set_standard_view(h, UInt32(view.rawValue))
    }

    public func frameAll() {
        guard let h = handle else { return }
        nova_core_camera_frame_all(h)
    }

    public func focusSelected() {
        guard let h = handle else { return }
        nova_core_camera_focus_selected(h)
    }

    // MARK: - Selection & Gizmo
    public func pickNode(ndcX: Float, ndcY: Float, aspect: Float, additive: Bool = false) {
        guard let h = handle else { return }
        var hitId: UInt64 = 0
        if nova_core_scene_pick_at_ndc(h, ndcX, ndcY, aspect, additive, &hitId) == 0 {
            refreshSceneState()
        }
    }

    public func pickGizmoAxis(ndcX: Float, ndcY: Float, aspect: Float) -> UInt32 {
        guard let h = handle else { return 0 }
        return nova_core_gizmo_pick_axis(h, ndcX, ndcY, aspect)
    }

    public func beginGizmoDrag(axis: UInt32, ndcX: Float, ndcY: Float, aspect: Float) -> Bool {
        guard let h = handle else { return false }
        return nova_core_gizmo_begin_drag(h, axis, ndcX, ndcY, aspect)
    }

    public func updateGizmoDrag(ndcX: Float, ndcY: Float, aspect: Float) -> Bool {
        guard let h = handle else { return false }
        let res = nova_core_gizmo_update_drag(h, ndcX, ndcY, aspect)
        if res {
            hasUnsavedChanges = true
            refreshSceneState()
        }
        return res
    }

    public func endGizmoDrag() {
        guard let h = handle else { return }
        nova_core_gizmo_end_drag(h)
        refreshSceneState()
    }

    public func setGizmoMode(_ mode: GizmoMode) {
        guard let h = handle else { return }
        self.gizmoMode = mode
        nova_core_gizmo_set_mode(h, UInt32(mode.rawValue))
    }

    // MARK: - Mode & Tools
    public func toggleInteractionMode() {
        guard let h = handle else { return }
        let nextMode: InteractionMode = (interactionMode == .object) ? .edit : .object
        self.interactionMode = nextMode
        nova_core_set_interaction_mode(h, UInt32(nextMode.rawValue))
        refreshSceneState()
    }

    public func setEditSelectMode(_ mode: EditSelectMode) {
        guard let h = handle else { return }
        self.editSelectMode = mode
        nova_core_edit_set_select_mode(h, UInt32(mode.rawValue))
    }

    public func extrude(distance: Float = 0.5) {
        guard let h = handle else { return }
        nova_core_edit_extrude(h, distance)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func bevel(width: Float = 0.2) {
        guard let h = handle else { return }
        nova_core_edit_bevel(h, width)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func subdivide() {
        guard let h = handle else { return }
        nova_core_edit_subdivide(h)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func loopCut() {
        guard let h = handle else { return }
        nova_core_edit_loop_cut(h)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func inset(factor: Float = 0.2) {
        guard let h = handle else { return }
        nova_core_edit_inset(h, factor)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func deleteSelected() {
        guard let h = handle else { return }
        if interactionMode == .edit {
            nova_core_edit_delete(h)
        } else {
            nova_core_scene_delete_selected(h)
        }
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func addPrimitive(type: UInt32) {
        guard let h = handle else { return }
        var outId: UInt64 = 0
        nova_core_scene_add_primitive(h, type, &outId)
        hasUnsavedChanges = true
        refreshSceneState()
    }

    public func undo() {
        guard let h = handle else { return }
        if nova_core_undo(h) {
            refreshSceneState()
        }
    }

    public func redo() {
        guard let h = handle else { return }
        if nova_core_redo(h) {
            refreshSceneState()
        }
    }

    public func showToast(_ msg: String) {
        self.statusMessage = msg
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) { [weak self] in
            if self?.statusMessage == msg {
                self?.statusMessage = nil
            }
        }
    }
}
