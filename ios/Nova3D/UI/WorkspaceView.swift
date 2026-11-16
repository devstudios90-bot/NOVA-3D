import SwiftUI

public struct WorkspaceView: View {
    @StateObject private var bridge = NovaBridge()
    @State private var activeWorkspaceTab: String = "Modeling"
    @State private var isPlaying: Bool = false
    @State private var currentFrame: Float = 0.0
    @State private var showSidePanel: Bool = true

    let tabs = ["Modeling", "Materials", "Animation", "Render", "Projects", "Settings"]

    public init() {}

    public var body: some View {
        ZStack {
            NovaTheme.canvasBg.ignoresSafeArea()

            VStack(spacing: 0) {
                // Top Header Bar
                HStack(spacing: 12) {
                    HStack(spacing: 8) {
                        NovaBrandMark(size: 24)
                        Text("NOVA")
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(NovaTheme.textPrimary)
                        Text("3D")
                            .font(.system(size: 14, weight: .bold))
                            .foregroundColor(NovaTheme.accentVioletGlow)
                    }

                    Rectangle()
                        .fill(NovaTheme.panelBorder)
                        .frame(width: 1, height: 18)

                    HStack(spacing: 6) {
                        Text(bridge.projectTitle)
                            .font(.system(size: 12, weight: .medium))
                            .foregroundColor(NovaTheme.textSecondary)

                        Circle()
                            .fill(bridge.hasUnsavedChanges ? NovaTheme.accentVioletGlow : NovaTheme.successGreen)
                            .frame(width: 6, height: 6)
                    }

                    // Workspace Switcher Pills
                    ScrollView(.horizontal, showsIndicators: false) {
                        HStack(spacing: 4) {
                            ForEach(tabs, id: \.self) { tab in
                                Button(action: { activeWorkspaceTab = tab }) {
                                    Text(tab)
                                        .font(.system(size: 11, weight: .medium))
                                        .padding(.horizontal, 10)
                                        .padding(.vertical, 5)
                                        .background(
                                            Capsule()
                                                .fill(activeWorkspaceTab == tab ? NovaTheme.accentViolet : Color.clear)
                                        )
                                        .foregroundColor(activeWorkspaceTab == tab ? .white : NovaTheme.textSecondary)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }

                    Spacer()

                    // Undo / Redo & Shading Actions
                    HStack(spacing: 8) {
                        Button(action: { bridge.undo() }) {
                            Image(systemName: "arrow.uturn.backward")
                                .font(.system(size: 13))
                                .foregroundColor(NovaTheme.textSecondary)
                        }
                        Button(action: { bridge.redo() }) {
                            Image(systemName: "arrow.uturn.forward")
                                .font(.system(size: 13))
                                .foregroundColor(NovaTheme.textSecondary)
                        }

                        Button(action: { showSidePanel.toggle() }) {
                            Image(systemName: "sidebar.right")
                                .font(.system(size: 14))
                                .foregroundColor(showSidePanel ? NovaTheme.accentVioletGlow : NovaTheme.textSecondary)
                        }
                    }
                }
                .padding(.horizontal, 12)
                .frame(height: 48)
                .background(NovaTheme.panelBg)
                .overlay(
                    Rectangle()
                        .fill(NovaTheme.panelBorder)
                        .frame(height: 1),
                    alignment: .bottom
                )

                // Main 3-Zone Workspace Body
                HStack(spacing: 0) {
                    // Zone 1: Left Vertical Tool Dock
                    VStack(spacing: 6) {
                        NovaDockButton(
                            iconName: "slider.horizontal.3",
                            active: bridge.interactionMode == .edit,
                            onClick: { bridge.toggleInteractionMode() }
                        )

                        Divider().background(NovaTheme.panelBorder).padding(.vertical, 2)

                        NovaDockButton(
                            iconName: "cursorarrow",
                            active: bridge.gizmoMode == .select,
                            onClick: { bridge.setGizmoMode(.select) }
                        )
                        NovaDockButton(
                            iconName: "arrow.up.and.down.and.arrow.left.and.right",
                            active: bridge.gizmoMode == .move,
                            onClick: { bridge.setGizmoMode(.move) }
                        )
                        NovaDockButton(
                            iconName: "rotate.right",
                            active: bridge.gizmoMode == .rotate,
                            onClick: { bridge.setGizmoMode(.rotate) }
                        )
                        NovaDockButton(
                            iconName: "arrow.up.left.and.down.right.and.arrow.up.right.and.down.left",
                            active: bridge.gizmoMode == .scale,
                            onClick: { bridge.setGizmoMode(.scale) }
                        )

                        Divider().background(NovaTheme.panelBorder).padding(.vertical, 2)

                        // Sub-element selections for Edit Mode
                        NovaDockButton(
                            iconName: "circle.fill",
                            active: bridge.editSelectMode == .vertex,
                            onClick: { bridge.setEditSelectMode(.vertex) }
                        )
                        NovaDockButton(
                            iconName: "line.diagonal",
                            active: bridge.editSelectMode == .edge,
                            onClick: { bridge.setEditSelectMode(.edge) }
                        )
                        NovaDockButton(
                            iconName: "square",
                            active: bridge.editSelectMode == .face,
                            onClick: { bridge.setEditSelectMode(.face) }
                        )

                        Divider().background(NovaTheme.panelBorder).padding(.vertical, 2)

                        // Primary modeling operators
                        NovaDockButton(iconName: "arrow.up.forward.app", onClick: { bridge.extrude() })
                        NovaDockButton(iconName: "skew", onClick: { bridge.bevel() })
                        NovaDockButton(iconName: "square.split.2x2", onClick: { bridge.subdivide() })
                        NovaDockButton(iconName: "circle.circle", onClick: { bridge.loopCut() })

                        Spacer()
                    }
                    .frame(width: 50)
                    .padding(.vertical, 6)
                    .background(NovaTheme.panelBg)
                    .overlay(
                        Rectangle()
                            .fill(NovaTheme.panelBorder)
                            .frame(width: 1),
                        alignment: .trailing
                    )

                    // Zone 2: Center 3D Metal Viewport
                    ZStack {
                        NovaMetalViewport(bridge: bridge)
                            .ignoresSafeArea()

                        // Top-right floating orientation cube
                        VStack {
                            HStack {
                                Spacer()
                                OrientationCubeView(bridge: bridge)
                                    .padding(12)
                            }
                            Spacer()
                        }

                        // Bottom docked timeline scrubber
                        VStack {
                            Spacer()
                            HStack(spacing: 12) {
                                Button(action: { currentFrame = 0 }) {
                                    Image(systemName: "backward.end.fill")
                                        .font(.system(size: 11))
                                        .foregroundColor(NovaTheme.textSecondary)
                                }
                                Button(action: { isPlaying.toggle() }) {
                                    Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                                        .font(.system(size: 12))
                                        .foregroundColor(NovaTheme.accentVioletGlow)
                                }

                                Text("00:00:15 / 00:00:30")
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundColor(NovaTheme.textMuted)

                                Slider(value: $currentFrame, in: 0...900)
                                    .accentColor(NovaTheme.accentViolet)

                                Text("Frame \(Int(currentFrame))")
                                    .font(.system(size: 10, weight: .bold, design: .monospaced))
                                    .foregroundColor(NovaTheme.textPrimary)
                            }
                            .padding(.horizontal, 16)
                            .frame(height: 40)
                            .background(NovaTheme.panelBg.opacity(0.92))
                            .overlay(
                                Rectangle()
                                    .fill(NovaTheme.panelBorder)
                                    .frame(height: 1),
                                alignment: .top
                            )
                        }
                    }

                    // Zone 3: Right Collapsible Accordion Properties Panel
                    if showSidePanel {
                        ScrollView {
                            VStack(alignment: .leading, spacing: 14) {
                                // Transform Card
                                VStack(alignment: .leading, spacing: 8) {
                                    Text("TRANSFORM")
                                        .font(.system(size: 10, weight: .bold))
                                        .foregroundColor(NovaTheme.accentVioletGlow)

                                    if let node = bridge.selectedNode {
                                        HStack {
                                            Text("Loc").font(.system(size: 11)).foregroundColor(NovaTheme.textMuted)
                                            Spacer()
                                            Text(String(format: "X: %.2f  Y: %.2f  Z: %.2f", node.position.0, node.position.1, node.position.2))
                                                .font(.system(size: 10, design: .monospaced))
                                                .foregroundColor(NovaTheme.textPrimary)
                                        }
                                        HStack {
                                            Text("Rot").font(.system(size: 11)).foregroundColor(NovaTheme.textMuted)
                                            Spacer()
                                            Text(String(format: "%.0f°  %.0f°  %.0f°", node.rotation.0, node.rotation.1, node.rotation.2))
                                                .font(.system(size: 10, design: .monospaced))
                                                .foregroundColor(NovaTheme.textPrimary)
                                        }
                                    } else {
                                        Text("No Selection")
                                            .font(.system(size: 11))
                                            .foregroundColor(NovaTheme.textMuted)
                                    }
                                }
                                .padding(10)
                                .background(NovaTheme.panelSurface)
                                .cornerRadius(8)

                                // Mesh Data
                                VStack(alignment: .leading, spacing: 6) {
                                    Text("MESH DATA")
                                        .font(.system(size: 10, weight: .bold))
                                        .foregroundColor(NovaTheme.accentCyan)
                                    HStack {
                                        Text("V: \(bridge.vertexCount)  E: \(bridge.edgeCount)  F: \(bridge.faceCount)")
                                            .font(.system(size: 10, design: .monospaced))
                                            .foregroundColor(NovaTheme.textSecondary)
                                    }
                                }
                                .padding(10)
                                .background(NovaTheme.panelSurface)
                                .cornerRadius(8)

                                // Outliner
                                VStack(alignment: .leading, spacing: 6) {
                                    Text("OUTLINER")
                                        .font(.system(size: 10, weight: .bold))
                                        .foregroundColor(NovaTheme.textMuted)

                                    ForEach(bridge.sceneNodes) { node in
                                        HStack {
                                            Image(systemName: node.hasMesh ? "cube" : "lightbulb")
                                                .font(.system(size: 11))
                                                .foregroundColor(node.primarySelected ? NovaTheme.accentVioletGlow : NovaTheme.textMuted)
                                            Text(node.name)
                                                .font(.system(size: 11))
                                                .foregroundColor(node.primarySelected ? .white : NovaTheme.textSecondary)
                                            Spacer()
                                        }
                                        .padding(.vertical, 3)
                                    }
                                }
                                .padding(10)
                                .background(NovaTheme.panelSurface)
                                .cornerRadius(8)
                            }
                            .padding(12)
                        }
                        .frame(width: 260)
                        .background(NovaTheme.panelBg)
                        .overlay(
                            Rectangle()
                                .fill(NovaTheme.panelBorder)
                                .frame(width: 1),
                            alignment: .leading
                        )
                    }
                }
            }

            // Toast status banner
            if let msg = bridge.statusMessage {
                VStack {
                    Spacer()
                    Text(msg)
                        .font(.system(size: 12, weight: .medium))
                        .foregroundColor(.white)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)
                        .background(Capsule().fill(NovaTheme.panelBg).overlay(Capsule().stroke(NovaTheme.accentViolet, lineWidth: 1)))
                        .padding(.bottom, 60)
                }
            }
        }
    }
}
