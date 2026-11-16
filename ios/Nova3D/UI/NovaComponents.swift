import SwiftUI

public struct NovaBrandMark: View {
    public var size: CGFloat = 24.0

    public init(size: CGFloat = 24.0) {
        self.size = size
    }

    public var body: some View {
        Canvas { context, sz in
            let w = sz.width
            let h = sz.height
            let cx = w * 0.5
            let cy = h * 0.5
            let r = min(w, h) * 0.42
            let cos30: CGFloat = 0.8660254
            let sin30: CGFloat = 0.5

            let top = CGPoint(x: cx, y: cy - r)
            let topRight = CGPoint(x: cx + r * cos30, y: cy - r * sin30)
            let botRight = CGPoint(x: cx + r * cos30, y: cy + r * sin30)
            let bot = CGPoint(x: cx, y: cy + r)
            let botLeft = CGPoint(x: cx - r * cos30, y: cy + r * sin30)
            let topLeft = CGPoint(x: cx - r * cos30, y: cy - r * sin30)
            let center = CGPoint(x: cx, y: cy)

            // Right solid face (Electric Violet gradient)
            var rightPath = Path()
            rightPath.move(to: center)
            rightPath.addLine(to: topRight)
            rightPath.addLine(to: botRight)
            rightPath.addLine(to: bot)
            rightPath.closeSubpath()
            context.fill(rightPath, with: .linearGradient(
                Gradient(colors: [NovaTheme.accentViolet, NovaTheme.accentVioletGlow]),
                startPoint: topRight,
                endPoint: bot
            ))

            // Top solid face (Cyan to Violet)
            var topPath = Path()
            topPath.move(to: top)
            topPath.addLine(to: topRight)
            topPath.addLine(to: center)
            topPath.addLine(to: CGPoint(x: (top.x + topLeft.x) * 0.5, y: (top.y + topLeft.y) * 0.5))
            topPath.closeSubpath()
            context.fill(topPath, with: .linearGradient(
                Gradient(colors: [NovaTheme.accentCyan, NovaTheme.accentViolet]),
                startPoint: top,
                endPoint: center
            ))

            // Wireframe lattice
            var wire = Path()
            let edges = [
                (topLeft, top), (topLeft, botLeft), (botLeft, bot),
                (topLeft, center), (botLeft, center), (top, center), (center, bot)
            ]
            for (p1, p2) in edges {
                wire.move(to: p1)
                wire.addLine(to: p2)
            }
            context.stroke(wire, with: .color(NovaTheme.accentCyan.opacity(0.85)), lineWidth: w * 0.05)

            // Outer silhouette
            var outline = Path()
            outline.move(to: top)
            outline.addLine(to: topRight)
            outline.addLine(to: botRight)
            outline.addLine(to: bot)
            outline.addLine(to: botLeft)
            outline.addLine(to: topLeft)
            outline.closeSubpath()
            context.stroke(outline, with: .color(.white.opacity(0.4)), lineWidth: w * 0.04)
        }
        .frame(width: size, height: size)
    }
}

public struct OrientationCubeView: View {
    @ObservedObject var bridge: NovaBridge
    public var size: CGFloat = 46.0

    public var body: some View {
        Menu {
            Button("Top View (+Z)") { bridge.setStandardView(.top) }
            Button("Front View (-Y)") { bridge.setStandardView(.front) }
            Button("Right View (+X)") { bridge.setStandardView(.right) }
            Button("Isometric View") { bridge.setStandardView(.isometric) }
            Divider()
            Button("Frame All") { bridge.frameAll() }
            Button("Focus Selected") { bridge.focusSelected() }
        } label: {
            ZStack {
                RoundedRectangle(cornerRadius: 10)
                    .fill(NovaTheme.panelBg.opacity(0.85))
                    .overlay(
                        RoundedRectangle(cornerRadius: 10)
                            .stroke(NovaTheme.panelBorder, lineWidth: 1)
                    )

                VStack(spacing: 1) {
                    Text("TOP")
                        .font(.system(size: 9, weight: .bold, design: .monospaced))
                        .foregroundColor(NovaTheme.axisY)
                    HStack(spacing: 4) {
                        Text("L")
                            .font(.system(size: 8, weight: .bold))
                            .foregroundColor(NovaTheme.axisX)
                        Text("R")
                            .font(.system(size: 8, weight: .bold))
                            .foregroundColor(NovaTheme.axisZ)
                    }
                }
            }
            .frame(width: size, height: size)
        }
    }
}

public struct NovaDockButton: View {
    public let iconName: String
    public let label: String?
    public let active: Bool
    public let onClick: () -> Void

    public init(iconName: String, label: String? = nil, active: Bool = false, onClick: @escaping () -> Void) {
        self.iconName = iconName
        self.label = label
        self.active = active
        self.onClick = onClick
    }

    public var body: some View {
        Button(action: onClick) {
            VStack(spacing: 2) {
                Image(systemName: iconName)
                    .font(.system(size: 16, weight: active ? .semibold : .regular))
                    .foregroundColor(active ? .white : NovaTheme.textSecondary)

                if let l = label {
                    Text(l)
                        .font(.system(size: 9, weight: .medium))
                        .foregroundColor(active ? .white : NovaTheme.textMuted)
                }
            }
            .frame(width: 40, height: 40)
            .background(
                RoundedRectangle(cornerRadius: 8)
                    .fill(active ? NovaTheme.accentViolet : Color.clear)
            )
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(active ? NovaTheme.accentVioletGlow.opacity(0.8) : Color.clear, lineWidth: 1)
            )
        }
        .buttonStyle(.plain)
    }
}
