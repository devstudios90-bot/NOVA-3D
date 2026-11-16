package com.nova3d.studio.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.KeyboardArrowUp
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.StandardViewPreset
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.roundToInt
import kotlin.math.sin

/**
 * Official NOVA 3D isometric cube mark:
 * Left half = cyan/violet wireframe lattice; Right half = solid beveled violet-cyan cube.
 */
@Composable
fun NovaBrandMark(
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
) {
    Canvas(modifier = modifier.size(size)) {
        val w = this.size.width
        val h = this.size.height
        val cx = w * 0.5f
        val cy = h * 0.5f
        val r = minOf(w, h) * 0.42f
        val cos30 = 0.8660254f
        val sin30 = 0.5f

        val top = Offset(cx, cy - r)
        val topRight = Offset(cx + r * cos30, cy - r * sin30)
        val botRight = Offset(cx + r * cos30, cy + r * sin30)
        val bot = Offset(cx, cy + r)
        val botLeft = Offset(cx - r * cos30, cy + r * sin30)
        val topLeft = Offset(cx - r * cos30, cy - r * sin30)
        val center = Offset(cx, cy)

        // Right solid face (Electric Violet -> Deep Magenta gradient)
        val rightFace = Path().apply {
            moveTo(center.x, center.y)
            lineTo(topRight.x, topRight.y)
            lineTo(botRight.x, botRight.y)
            lineTo(bot.x, bot.y)
            close()
        }
        drawPath(
            path = rightFace,
            brush = Brush.linearGradient(
                colors = listOf(Color(0xFF7B5CFF), Color(0xFFA855F7)),
                start = topRight,
                end = bot,
            ),
        )

        // Top solid half-facet (Cyan -> Violet)
        val topFace = Path().apply {
            moveTo(top.x, top.y)
            lineTo(topRight.x, topRight.y)
            lineTo(center.x, center.y)
            lineTo((top.x + topLeft.x) * 0.5f, (top.y + topLeft.y) * 0.5f)
            close()
        }
        drawPath(
            path = topFace,
            brush = Brush.linearGradient(
                colors = listOf(NovaColors.AccentCyan, NovaColors.AccentViolet),
                start = top,
                end = center,
            ),
        )

        // Inner dark bevel cutout on right face for 3D depth
        val innerCenter = Offset(
            (center.x + botRight.x + topRight.x + bot.x) * 0.25f,
            (center.y + botRight.y + topRight.y + bot.y) * 0.25f,
        )
        val innerHole = Path().apply {
            fun lerpToInner(p: Offset) = Offset(
                p.x + (innerCenter.x - p.x) * 0.52f,
                p.y + (innerCenter.y - p.y) * 0.52f,
            )
            val p1 = lerpToInner(center)
            val p2 = lerpToInner(topRight)
            val p3 = lerpToInner(botRight)
            val p4 = lerpToInner(bot)
            moveTo(p1.x, p1.y)
            lineTo(p2.x, p2.y)
            lineTo(p3.x, p3.y)
            lineTo(p4.x, p4.y)
            close()
        }
        drawPath(innerHole, color = NovaColors.CanvasBg)
        drawPath(
            innerHole,
            color = NovaColors.AccentCyan.copy(alpha = 0.7f),
            style = Stroke(width = w * 0.04f, join = StrokeJoin.Round),
        )

        // Left wireframe lattice
        val wireStroke = w * 0.055f
        val wireColor = NovaColors.AccentCyan.copy(alpha = 0.9f)
        val latticeEdges = listOf(
            topLeft to top,
            topLeft to botLeft,
            botLeft to bot,
            topLeft to center,
            botLeft to center,
            top to center,
            center to bot,
            Offset((topLeft.x + botLeft.x) * 0.5f, (topLeft.y + botLeft.y) * 0.5f) to center,
            Offset((topLeft.x + top.x) * 0.5f, (topLeft.y + top.y) * 0.5f) to Offset((botLeft.x + bot.x) * 0.5f, (botLeft.y + bot.y) * 0.5f),
        )
        latticeEdges.forEach { (a, b) ->
            drawLine(
                color = wireColor,
                start = a,
                end = b,
                strokeWidth = wireStroke,
                cap = StrokeCap.Round,
            )
        }

        // Outer crisp silhouette outline
        val outline = Path().apply {
            moveTo(top.x, top.y)
            lineTo(topRight.x, topRight.y)
            lineTo(botRight.x, botRight.y)
            lineTo(bot.x, bot.y)
            lineTo(botLeft.x, botLeft.y)
            lineTo(topLeft.x, topLeft.y)
            close()
        }
        drawPath(
            path = outline,
            color = Color.White.copy(alpha = 0.35f),
            style = Stroke(width = w * 0.04f, join = StrokeJoin.Round),
        )

        // glowing vertex dots on lattice
        listOf(top, topLeft, botLeft, center, bot).forEach { pt ->
            drawCircle(
                color = NovaColors.AccentCyan,
                radius = w * 0.065f,
                center = pt,
            )
        }
    }
}

/**
 * Interactive 3D-projected Orientation Cube widget for Viewport top-right corner.
 * Rotates dynamically with camera yaw & pitch and opens standard view presets on tap.
 */
@Composable
fun OrientationCubeWidget(
    yawDeg: Float,
    pitchDeg: Float,
    onSelectViewPreset: (StandardViewPreset) -> Unit,
    modifier: Modifier = Modifier,
) {
    var menuExpanded by remember { mutableStateOf(false) }

    Box(modifier = modifier) {
        Surface(
            color = NovaColors.PanelBg.copy(alpha = 0.78f),
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder.copy(alpha = 0.85f)),
            modifier = Modifier
                .size(50.dp)
                .clip(RoundedCornerShape(12.dp))
                .clickable { menuExpanded = true },
        ) {
            Canvas(modifier = Modifier.fillMaxSize().padding(6.dp)) {
                val cx = size.width * 0.5f
                val cy = size.height * 0.52f
                val r = minOf(size.width, size.height) * 0.33f

                val yaw = (yawDeg * PI / 180.0).toFloat()
                val pitch = (pitchDeg * PI / 180.0).toFloat()
                val cosY = cos(yaw); val sinY = sin(yaw)
                val cosP = cos(pitch); val sinP = sin(pitch)

                fun project(x: Float, y: Float, z: Float): Pair<Offset, Float> {
                    // Rotate around Y (yaw), then around X (pitch)
                    val x1 = x * cosY - z * sinY
                    val z1 = x * sinY + z * cosY
                    val y2 = y * cosP - z1 * sinP
                    val z2 = y * sinP + z1 * cosP
                    return Offset(cx + x1 * r, cy - y2 * r) to z2
                }

                val corners = arrayOf(
                    floatArrayOf(-1f, -1f, -1f),
                    floatArrayOf(1f, -1f, -1f),
                    floatArrayOf(1f, 1f, -1f),
                    floatArrayOf(-1f, 1f, -1f),
                    floatArrayOf(-1f, -1f, 1f),
                    floatArrayOf(1f, -1f, 1f),
                    floatArrayOf(1f, 1f, 1f),
                    floatArrayOf(-1f, 1f, 1f),
                )
                val proj = corners.map { project(it[0], it[1], it[2]) }

                val faces = listOf(
                    Triple(intArrayOf(4, 5, 6, 7), Color(0xFF3B82F6), "Z"), // +Z Front
                    Triple(intArrayOf(1, 0, 3, 2), Color(0xFF282C3D), "-Z"),
                    Triple(intArrayOf(5, 1, 2, 6), Color(0xFFEF4444), "X"), // +X Right
                    Triple(intArrayOf(0, 4, 7, 3), Color(0xFF362129), "-X"),
                    Triple(intArrayOf(3, 2, 6, 7), Color(0xFF22C55E), "Y"), // +Y Top
                    Triple(intArrayOf(0, 1, 5, 4), Color(0xFF1E222E), "-Y"),
                )

                val sortedFaces = faces.map { (idx, col, lbl) ->
                    val avgZ = idx.map { proj[it].second }.average().toFloat()
                    Triple(idx, col.copy(alpha = 0.72f), lbl) to avgZ
                }.sortedBy { it.second }

                // Only draw front-facing faces (highest avgZ)
                sortedFaces.takeLast(3).forEach { (faceInfo, _) ->
                    val (idx, col, _) = faceInfo
                    val p = Path().apply {
                        moveTo(proj[idx[0]].first.x, proj[idx[0]].first.y)
                        lineTo(proj[idx[1]].first.x, proj[idx[1]].first.y)
                        lineTo(proj[idx[2]].first.x, proj[idx[2]].first.y)
                        lineTo(proj[idx[3]].first.x, proj[idx[3]].first.y)
                        close()
                    }
                    drawPath(p, color = col)
                    drawPath(
                        p,
                        color = Color.White.copy(alpha = 0.55f),
                        style = Stroke(width = 1.4f, join = StrokeJoin.Round),
                    )
                }

                // Draw primary X, Y, Z axes rays
                val origin = project(0f, 0f, 0f).first
                val xTip = project(1.45f, 0f, 0f).first
                val yTip = project(0f, 1.45f, 0f).first
                val zTip = project(0f, 0f, 1.45f).first

                drawLine(NovaColors.AxisX, origin, xTip, strokeWidth = 2.4f, cap = StrokeCap.Round)
                drawLine(NovaColors.AxisY, origin, yTip, strokeWidth = 2.4f, cap = StrokeCap.Round)
                drawLine(NovaColors.AxisZ, origin, zTip, strokeWidth = 2.4f, cap = StrokeCap.Round)
            }
        }

        DropdownMenu(
            expanded = menuExpanded,
            onDismissRequest = { menuExpanded = false },
            containerColor = NovaColors.PanelBg,
        ) {
            StandardViewPreset.entries.forEach { preset ->
                DropdownMenuItem(
                    text = {
                        Text(
                            text = "${preset.label} View",
                            color = NovaColors.TextPrimary,
                            fontSize = 12.sp,
                        )
                    },
                    onClick = {
                        menuExpanded = false
                        onSelectViewPreset(preset)
                    },
                )
            }
        }
    }
}

/**
 * Compact professional icon button used inside tool docks and left tool rail.
 */
@Composable
fun NovaDockToolButton(
    icon: ImageVector,
    label: String?,
    active: Boolean,
    enabled: Boolean = true,
    accentColor: Color = NovaColors.AccentViolet,
    compact: Boolean = false,
    onClick: () -> Unit,
) {
    val bgColor = when {
        !enabled -> Color.Transparent
        active -> accentColor
        else -> Color.Transparent
    }
    val contentColor = when {
        !enabled -> NovaColors.TextMuted.copy(alpha = 0.45f)
        active -> Color.White
        else -> NovaColors.TextSecondary
    }

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .clip(RoundedCornerShape(10.dp))
            .background(bgColor)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(
                horizontal = if (compact) 6.dp else 8.dp,
                vertical = if (compact) 5.dp else 6.dp,
            ),
    ) {
        Icon(
            imageVector = icon,
            contentDescription = label ?: "Tool",
            tint = contentColor,
            modifier = Modifier.size(if (compact) 18.dp else 20.dp),
        )
        if (!label.isNullOrBlank()) {
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = label,
                color = if (active) Color.White else contentColor,
                fontSize = if (compact) 9.sp else 10.sp,
                fontWeight = if (active) FontWeight.SemiBold else FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/**
 * Action pill button in the floating right viewport rail.
 */
@Composable
fun ViewportActionIcon(
    icon: ImageVector,
    contentDescription: String,
    active: Boolean = false,
    subtleActive: Boolean = false,
    compact: Boolean = false,
    onClick: () -> Unit,
) {
    val bg = when {
        active -> NovaColors.AccentViolet
        subtleActive -> NovaColors.AccentVioletSoft.copy(alpha = 0.75f)
        else -> Color.Transparent
    }
    val tint = when {
        active -> Color.White
        subtleActive -> NovaColors.AccentCyan
        else -> NovaColors.TextSecondary
    }
    Box(
        modifier = Modifier
            .size(if (compact) 30.dp else 34.dp)
            .clip(RoundedCornerShape(8.dp))
            .background(bg)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = contentDescription,
            tint = tint,
            modifier = Modifier.size(if (compact) 16.dp else 18.dp),
        )
    }
}

/**
 * Clean Landscape Bottom Timeline Strip matching Reference #3:
 * Play/Pause, Rewind, Next, Key, Frame pill + Ruler with keyframe diamonds & purple playhead thumb.
 */
@Composable
fun LandscapeTimelineStrip(
    currentFrame: Float,
    startFrame: Int,
    endFrame: Int,
    isPlaying: Boolean,
    autoKey: Boolean,
    keyframeFrames: Set<Int>,
    onRewind: () -> Unit,
    onTogglePlay: () -> Unit,
    onStepNextKey: () -> Unit,
    onAddKey: () -> Unit,
    onToggleAutoKey: () -> Unit,
    onScrubFrame: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = NovaColors.PanelBg,
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = modifier.fillMaxWidth().height(42.dp),
    ) {
        Row(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Transport controls
            Surface(
                color = NovaColors.CardBg,
                shape = RoundedCornerShape(8.dp),
                border = BorderStroke(1.dp, NovaColors.PanelBorder),
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 4.dp, vertical = 3.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    TimelineTransportBtn("|◀", false, onRewind)
                    TimelineTransportBtn(if (isPlaying) "⏸" else "▶", isPlaying, onTogglePlay)
                    TimelineTransportBtn("▶|", false, onStepNextKey)
                }
            }

            // Frame indicator
            Text(
                text = "Frame ${currentFrame.roundToInt()}",
                color = NovaColors.TextPrimary,
                fontSize = 11.sp,
                fontWeight = FontWeight.SemiBold,
                fontFamily = FontFamily.Monospace,
            )

            // Quick Key & Auto-Key chips
            Surface(
                color = NovaColors.CardBg,
                shape = RoundedCornerShape(6.dp),
                border = BorderStroke(1.dp, NovaColors.PanelBorder),
                modifier = Modifier.clip(RoundedCornerShape(6.dp)).clickable(onClick = onAddKey),
            ) {
                Text(
                    text = "+◆",
                    color = NovaColors.WarningAmber,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    modifier = Modifier.padding(horizontal = 7.dp, vertical = 4.dp),
                )
            }
            Surface(
                color = if (autoKey) NovaColors.ErrorRed.copy(alpha = 0.22f) else NovaColors.CardBg,
                shape = RoundedCornerShape(6.dp),
                border = BorderStroke(1.dp, if (autoKey) NovaColors.ErrorRed else NovaColors.PanelBorder),
                modifier = Modifier.clip(RoundedCornerShape(6.dp)).clickable(onClick = onToggleAutoKey),
            ) {
                Text(
                    text = if (autoKey) "●Auto" else "Auto",
                    color = if (autoKey) NovaColors.ErrorRed else NovaColors.TextMuted,
                    fontSize = 10.sp,
                    fontWeight = FontWeight.Medium,
                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
                )
            }

            // Interactive Scrubber Ruler Canvas
            TimelineRulerScrubber(
                startFrame = startFrame,
                endFrame = endFrame,
                currentFrame = currentFrame,
                keyframeFrames = keyframeFrames,
                onScrubFrame = onScrubFrame,
                modifier = Modifier.weight(1f).height(38.dp),
            )
        }
    }
}

@Composable
private fun TimelineTransportBtn(
    label: String,
    active: Boolean,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(6.dp))
            .background(if (active) NovaColors.AccentViolet else Color.Transparent)
            .clickable(onClick = onClick)
            .padding(horizontal = 8.dp, vertical = 4.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            color = if (active) Color.White else NovaColors.TextPrimary,
            fontSize = 11.sp,
            fontWeight = FontWeight.SemiBold,
        )
    }
}

@Composable
private fun TimelineRulerScrubber(
    startFrame: Int,
    endFrame: Int,
    currentFrame: Float,
    keyframeFrames: Set<Int>,
    onScrubFrame: (Float) -> Unit,
    modifier: Modifier = Modifier,
) {
    var sizePx by remember { mutableStateOf(IntSize.Zero) }
    val textPaint = remember {
        android.graphics.Paint().apply {
            color = android.graphics.Color.argb(190, 148, 163, 184)
            textSize = 21f
            isAntiAlias = true
            typeface = android.graphics.Typeface.MONOSPACE
        }
    }
    val pillPaint = remember {
        android.graphics.Paint().apply {
            color = android.graphics.Color.WHITE
            textSize = 22f
            isAntiAlias = true
            textAlign = android.graphics.Paint.Align.CENTER
            typeface = android.graphics.Typeface.create(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD)
        }
    }

    Canvas(
        modifier = modifier
            .clip(RoundedCornerShape(8.dp))
            .background(NovaColors.CanvasBg)
            .border(1.dp, NovaColors.PanelBorder, RoundedCornerShape(8.dp))
            .onSizeChanged { sizePx = it }
            .pointerInput(startFrame, endFrame) {
                detectTapGestures { offset ->
                    val span = (endFrame - startFrame).coerceAtLeast(1)
                    val pad = 16f
                    val usable = (sizePx.width - pad * 2f).coerceAtLeast(1f)
                    val frac = ((offset.x - pad) / usable).coerceIn(0f, 1f)
                    onScrubFrame((startFrame + frac * span).roundToInt().toFloat())
                }
            }
            .pointerInput(startFrame, endFrame) {
                detectDragGestures { change, _ ->
                    change.consume()
                    val span = (endFrame - startFrame).coerceAtLeast(1)
                    val pad = 16f
                    val usable = (sizePx.width - pad * 2f).coerceAtLeast(1f)
                    val frac = ((change.position.x - pad) / usable).coerceIn(0f, 1f)
                    onScrubFrame((startFrame + frac * span).roundToInt().toFloat())
                }
            },
    ) {
        val span = (endFrame - startFrame).coerceAtLeast(1).toFloat()
        val pad = 16f
        val usableW = (size.width - pad * 2f).coerceAtLeast(1f)
        val h = size.height

        // Tick marks
        val step = if (span > 150f) 20 else 10
        val minorStep = if (span > 150f) 5 else 2
        for (f in startFrame..endFrame step minorStep) {
            val x = pad + ((f - startFrame) / span) * usableW
            val isMajor = (f % step == 0) || f == startFrame
            drawLine(
                color = if (isMajor) NovaColors.TextSecondary.copy(alpha = 0.65f) else NovaColors.PanelBorder,
                start = Offset(x, h * 0.56f),
                end = Offset(x, if (isMajor) h * 0.92f else h * 0.78f),
                strokeWidth = if (isMajor) 2f else 1f,
            )
            if (isMajor && x + 26f < size.width) {
                drawContext.canvas.nativeCanvas.drawText(
                    f.toString(),
                    x + 3f,
                    h * 0.46f,
                    textPaint,
                )
            }
        }

        // Keyframe diamonds on track
        keyframeFrames.forEach { kf ->
            if (kf in startFrame..endFrame) {
                val kx = pad + ((kf - startFrame) / span) * usableW
                val ky = h * 0.25f
                val d = 5.5f
                val path = Path().apply {
                    moveTo(kx, ky - d)
                    lineTo(kx + d, ky)
                    lineTo(kx, ky + d)
                    lineTo(kx - d, ky)
                    close()
                }
                drawPath(path, color = NovaColors.WarningAmber)
            }
        }

        // Playhead line + purple frame pill
        val playX = pad + ((currentFrame.coerceIn(startFrame.toFloat(), endFrame.toFloat()) - startFrame) / span) * usableW
        drawLine(
            color = NovaColors.AccentViolet,
            start = Offset(playX, 0f),
            end = Offset(playX, h),
            strokeWidth = 3f,
        )
        val pillW = 36f
        val pillH = 22f
        drawRoundRect(
            color = NovaColors.AccentViolet,
            topLeft = Offset((playX - pillW / 2f).coerceIn(2f, size.width - pillW - 2f), 2f),
            size = Size(pillW, pillH),
            cornerRadius = CornerRadius(6f, 6f),
        )
        drawContext.canvas.nativeCanvas.drawText(
            currentFrame.roundToInt().toString(),
            playX.coerceIn(pillW / 2f + 2f, size.width - pillW / 2f - 2f),
            19f,
            pillPaint,
        )
    }
}

/**
 * Collapsible accordion section header used in the Landscape Right Properties Panel.
 */
@Composable
fun NovaAccordionSection(
    title: String,
    expanded: Boolean,
    onToggle: () -> Unit,
    badge: String? = null,
    content: @Composable () -> Unit,
) {
    Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Column {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable(onClick = onToggle)
                    .padding(horizontal = 10.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Text(
                        text = title.uppercase(),
                        color = NovaColors.TextPrimary,
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.6.sp,
                    )
                    if (!badge.isNullOrBlank()) {
                        Surface(
                            color = NovaColors.AccentVioletSoft,
                            shape = RoundedCornerShape(4.dp),
                        ) {
                            Text(
                                text = badge,
                                color = NovaColors.AccentVioletGlow,
                                fontSize = 9.sp,
                                fontFamily = FontFamily.Monospace,
                                modifier = Modifier.padding(horizontal = 5.dp, vertical = 1.dp),
                            )
                        }
                    }
                }
                Icon(
                    imageVector = if (expanded) Icons.Rounded.KeyboardArrowUp else Icons.Rounded.KeyboardArrowDown,
                    contentDescription = if (expanded) "Collapse" else "Expand",
                    tint = NovaColors.TextSecondary,
                    modifier = Modifier.size(16.dp),
                )
            }

            AnimatedVisibility(visible = expanded) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(start = 10.dp, end = 10.dp, bottom = 10.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    content()
                }
            }
        }
    }
}
