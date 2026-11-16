package com.nova3d.studio.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.AutoAwesome
import androidx.compose.material.icons.rounded.CameraAlt
import androidx.compose.material.icons.rounded.Category
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.Edit
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.KeyboardArrowUp
import androidx.compose.material.icons.rounded.LightMode
import androidx.compose.material.icons.rounded.Palette
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material.icons.rounded.Videocam
import androidx.compose.material.icons.rounded.ViewInAr
import androidx.compose.material.icons.rounded.Visibility
import androidx.compose.material.icons.rounded.VisibilityOff
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.ui.components.NovaAccordionSection
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.EditSelectUiMode
import com.nova3d.studio.viewport.EnvironmentUiPreset
import com.nova3d.studio.viewport.InteractionUiMode
import com.nova3d.studio.viewport.LightingRigPreset
import com.nova3d.studio.viewport.MaterialUiPreset
import com.nova3d.studio.viewport.ModifierUiKind
import com.nova3d.studio.viewport.ModifierUiSpec
import com.nova3d.studio.viewport.NodeDetails
import com.nova3d.studio.viewport.SceneNodeSummary
import com.nova3d.studio.viewport.ShadingUiMode
import com.nova3d.studio.viewport.ViewportController
import java.util.Locale

private val MATERIAL_SWATCHES = listOf(
    floatArrayOf(0.84f, 0.86f, 0.90f, 1.0f),
    floatArrayOf(0.48f, 0.36f, 0.98f, 1.0f),
    floatArrayOf(0.22f, 0.74f, 0.97f, 1.0f),
    floatArrayOf(0.92f, 0.31f, 0.28f, 1.0f),
    floatArrayOf(0.96f, 0.64f, 0.20f, 1.0f),
    floatArrayOf(0.20f, 0.78f, 0.45f, 1.0f),
    floatArrayOf(0.68f, 0.42f, 0.95f, 1.0f),
    floatArrayOf(0.18f, 0.20f, 0.25f, 1.0f),
)

private val LIGHT_SWATCHES = listOf(
    floatArrayOf(1.0f, 0.98f, 0.94f),
    floatArrayOf(1.0f, 0.86f, 0.65f),
    floatArrayOf(0.75f, 0.88f, 1.0f),
    floatArrayOf(1.0f, 0.55f, 0.45f),
    floatArrayOf(0.50f, 0.95f, 0.72f),
)

private enum class InspectorTab {
    Outliner,
    Properties,
    World,
}

private enum class PortraitCardTab {
    Transform,
    Material,
}

/**
 * Landscape 3-Zone Right Collapsible Properties Panel with Accordion Sections
 * matching Reference #3: TRANSFORM, MESH, MATERIALS, MODIFIERS, WORLD / LIGHTING, OUTLINER.
 */
@Composable
fun LandscapeAccordionPropertiesPanel(
    controller: ViewportController,
    modifier: Modifier = Modifier,
) {
    val details = controller.selectedNodeDetails
    val summary = controller.sceneNodes.firstOrNull { it.id == details?.id }

    var expandTransform by remember { mutableStateOf(true) }
    var expandMesh by remember { mutableStateOf(false) }
    var expandMaterials by remember { mutableStateOf(true) }
    var expandModifiers by remember { mutableStateOf(false) }
    var expandWorld by remember { mutableStateOf(false) }
    var expandOutliner by remember { mutableStateOf(true) }

    var renameDialogOpen by remember(details?.id) { mutableStateOf(false) }
    var editingTransformField by remember { mutableStateOf<TransformEditTarget?>(null) }

    Surface(
        color = NovaColors.PanelBg,
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = modifier,
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Selected Node Header Card
            if (details != null && summary != null) {
                Surface(
                    color = NovaColors.CardBg,
                    shape = RoundedCornerShape(10.dp),
                    border = BorderStroke(1.dp, NovaColors.AccentViolet.copy(alpha = 0.45f)),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Box(
                            modifier = Modifier
                                .size(28.dp)
                                .clip(RoundedCornerShape(7.dp))
                                .background(NovaColors.AccentVioletSoft),
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                imageVector = when (details.kind) {
                                    1 -> Icons.Rounded.ViewInAr
                                    2 -> Icons.Rounded.LightMode
                                    3 -> Icons.Rounded.Videocam
                                    else -> Icons.Rounded.Category
                                },
                                contentDescription = null,
                                tint = NovaColors.AccentVioletGlow,
                                modifier = Modifier.size(16.dp),
                            )
                        }
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = summary.name,
                                color = NovaColors.TextPrimary,
                                fontSize = 12.sp,
                                fontWeight = FontWeight.SemiBold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                text = when (details.kind) {
                                    1 -> "Mesh Object"
                                    2 -> "Light Node"
                                    3 -> "Camera Node"
                                    else -> "Empty / Group"
                                },
                                color = NovaColors.TextMuted,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                            )
                        }
                        Icon(
                            imageVector = Icons.Rounded.Edit,
                            contentDescription = "Rename",
                            tint = NovaColors.TextSecondary,
                            modifier = Modifier
                                .size(18.dp)
                                .clip(RoundedCornerShape(4.dp))
                                .clickable { renameDialogOpen = true },
                        )
                        Icon(
                            imageVector = if (details.visible) Icons.Rounded.Visibility else Icons.Rounded.VisibilityOff,
                            contentDescription = "Toggle Visibility",
                            tint = if (details.visible) NovaColors.AccentCyan else NovaColors.TextMuted,
                            modifier = Modifier
                                .size(18.dp)
                                .clip(RoundedCornerShape(4.dp))
                                .clickable { controller.setNodeVisibility(details.id, !details.visible) },
                        )
                    }
                }
            } else {
                Surface(
                    color = NovaColors.CardBg,
                    shape = RoundedCornerShape(10.dp),
                    border = BorderStroke(1.dp, NovaColors.PanelBorder),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(
                        text = "No object selected · Tap viewport or Outliner",
                        color = NovaColors.TextMuted,
                        fontSize = 11.sp,
                        modifier = Modifier.padding(10.dp),
                    )
                }
            }

            // 1. TRANSFORM Accordion
            if (details != null) {
                NovaAccordionSection(
                    title = "Transform",
                    expanded = expandTransform,
                    onToggle = { expandTransform = !expandTransform },
                ) {
                    VectorRow(
                        label = "Pos",
                        x = details.posX,
                        y = details.posY,
                        z = details.posZ,
                        onEditAxis = { axis ->
                            editingTransformField = TransformEditTarget("Location", axis, details)
                        },
                    )
                    VectorRow(
                        label = "Rot",
                        x = details.rotX,
                        y = details.rotY,
                        z = details.rotZ,
                        onEditAxis = { axis ->
                            editingTransformField = TransformEditTarget("Rotation (°)", axis, details)
                        },
                    )
                    VectorRow(
                        label = "Scl",
                        x = details.scaleX,
                        y = details.scaleY,
                        z = details.scaleZ,
                        onEditAxis = { axis ->
                            editingTransformField = TransformEditTarget("Scale", axis, details)
                        },
                    )
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(5.dp),
                    ) {
                        MiniActionChip("Key ◆", false) { controller.keySelectedNodeTrs(7) }
                        MiniActionChip("Reset", false) {
                            controller.setNodeTransform(details.id, 0f, 0f, 0f, 0f, 0f, 0f, 1f, 1f, 1f)
                        }
                        if (details.hasMesh) {
                            MiniActionChip("Apply", false) { controller.applyTransformSelected() }
                            MiniActionChip("Origin", false) { controller.originToGeometrySelected() }
                        }
                    }
                }

                // 2. MESH Accordion (when object hasMesh)
                if (details.hasMesh) {
                    val inEdit = controller.editState.interactionMode == InteractionUiMode.Edit
                    NovaAccordionSection(
                        title = "Mesh",
                        badge = if (inEdit) "EDIT" else null,
                        expanded = expandMesh || inEdit,
                        onToggle = { expandMesh = !expandMesh },
                    ) {
                        if (inEdit) {
                            EditModelingPropertiesCard(controller = controller, embedInline = true)
                        } else {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    text = if (controller.editState.smoothShading) "Shading: Smooth" else "Shading: Flat",
                                    color = NovaColors.TextSecondary,
                                    fontSize = 11.sp,
                                )
                                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                                    MiniActionChip("Smooth", controller.editState.smoothShading) {
                                        controller.editSetSmoothShading(true)
                                    }
                                    MiniActionChip("Flat", !controller.editState.smoothShading) {
                                        controller.editSetSmoothShading(false)
                                    }
                                }
                            }
                            MiniActionChip(
                                label = "Enter Edit Mode",
                                active = true,
                                onClick = { controller.setInteractionMode(InteractionUiMode.Edit) },
                            )
                        }
                    }

                    // 3. MATERIALS Accordion
                    NovaAccordionSection(
                        title = "Materials",
                        badge = controller.materials.size.toString(),
                        expanded = expandMaterials,
                        onToggle = { expandMaterials = !expandMaterials },
                    ) {
                        // Sphere Preview Swatch Row (matching Reference #3!)
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            controller.materials.forEach { mat ->
                                val isAssigned = mat.id == details.materialId
                                Box(
                                    modifier = Modifier
                                        .size(34.dp)
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(
                                            Brush.radialGradient(
                                                colors = listOf(
                                                    Color(
                                                        (mat.r * 1.25f).coerceAtMost(1f),
                                                        (mat.g * 1.25f).coerceAtMost(1f),
                                                        (mat.b * 1.25f).coerceAtMost(1f),
                                                    ),
                                                    Color(mat.r * 0.4f, mat.g * 0.4f, mat.b * 0.4f),
                                                ),
                                            ),
                                        )
                                        .border(
                                            width = if (isAssigned) 2.dp else 1.dp,
                                            color = if (isAssigned) NovaColors.AccentViolet else NovaColors.PanelBorder,
                                            shape = RoundedCornerShape(8.dp),
                                        )
                                        .clickable { controller.assignMaterialToSelected(mat.id) },
                                )
                            }
                        }
                        MaterialPropertiesCard(controller = controller, details = details, embedInline = true)
                    }

                    // 4. MODIFIERS Accordion
                    NovaAccordionSection(
                        title = "Modifiers",
                        badge = if (controller.modifiers.isNotEmpty()) controller.modifiers.size.toString() else null,
                        expanded = expandModifiers,
                        onToggle = { expandModifiers = !expandModifiers },
                    ) {
                        ModifierStackCard(controller = controller, embedInline = true)
                    }
                }

                // Light / Camera contextual accordion
                if (details.kind == 2) {
                    NovaAccordionSection(
                        title = "Light Parameters",
                        expanded = true,
                        onToggle = {},
                    ) {
                        LightPropertiesCard(controller = controller, details = details, embedInline = true)
                    }
                }
                if (details.kind == 3) {
                    NovaAccordionSection(
                        title = "Camera Lens & View",
                        expanded = true,
                        onToggle = {},
                    ) {
                        CameraPropertiesCard(controller = controller, details = details, embedInline = true)
                    }
                }
            }

            // 5. WORLD / LIGHTING Accordion
            NovaAccordionSection(
                title = "World / Lighting",
                expanded = expandWorld,
                onToggle = { expandWorld = !expandWorld },
            ) {
                WorldLightingTabContent(
                    controller = controller,
                    embedInline = true,
                    modifier = Modifier.fillMaxWidth(),
                )
            }

            // 6. OUTLINER Accordion
            NovaAccordionSection(
                title = "Outliner",
                badge = controller.sceneNodes.size.toString(),
                expanded = expandOutliner,
                onToggle = { expandOutliner = !expandOutliner },
            ) {
                OutlinerTabContent(
                    controller = controller,
                    onJumpToProperties = { expandTransform = true },
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = 120.dp, max = 260.dp),
                )
            }
        }
    }

    if (renameDialogOpen && summary != null) {
        RenameNodeDialog(
            initialName = summary.name,
            onDismiss = { renameDialogOpen = false },
            onConfirm = { newName ->
                renameDialogOpen = false
                controller.renameNode(summary.id, newName)
            },
        )
    }

    val target = editingTransformField
    if (target != null) {
        NumericComponentDialog(
            target = target,
            onDismiss = { editingTransformField = null },
            onConfirm = { newVal ->
                editingTransformField = null
                applyTransformFieldUpdate(controller, target, newVal)
            },
        )
    }
}

/**
 * Portrait Floating Object Inspector Card matching Reference #1:
 * Compact floating card above the tool dock with Transform/Material tabs, numeric chips + sliders,
 * and quick Modifiers/Materials shortcuts.
 */
@Composable
fun PortraitFloatingObjectCard(
    controller: ViewportController,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    onOpenModifiersModal: () -> Unit,
    onOpenMaterialsModal: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val details = controller.selectedNodeDetails ?: return
    val summary = controller.sceneNodes.firstOrNull { it.id == details.id } ?: return

    var activeTab by remember { mutableStateOf(PortraitCardTab.Transform) }
    var editingTransformField by remember { mutableStateOf<TransformEditTarget?>(null) }
    var renameDialogOpen by remember(details.id) { mutableStateOf(false) }

    Surface(
        color = NovaColors.PanelBg.copy(alpha = 0.94f),
        shape = RoundedCornerShape(16.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        shadowElevation = 10.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Card Header: Cube Icon + Name + Collapse Chevron
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    modifier = Modifier
                        .weight(1f)
                        .clickable { renameDialogOpen = true },
                ) {
                    Icon(
                        imageVector = when (details.kind) {
                            1 -> Icons.Rounded.ViewInAr
                            2 -> Icons.Rounded.LightMode
                            3 -> Icons.Rounded.Videocam
                            else -> Icons.Rounded.Category
                        },
                        contentDescription = null,
                        tint = NovaColors.AccentVioletGlow,
                        modifier = Modifier.size(18.dp),
                    )
                    Text(
                        text = summary.name,
                        color = NovaColors.TextPrimary,
                        fontSize = 14.sp,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }

                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    // Quick Keyframe button
                    Surface(
                        color = NovaColors.CardBg,
                        shape = RoundedCornerShape(6.dp),
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        modifier = Modifier
                            .clip(RoundedCornerShape(6.dp))
                            .clickable { controller.keySelectedNodeTrs(7) },
                    ) {
                        Text(
                            text = "Key ◆",
                            color = NovaColors.WarningAmber,
                            fontSize = 10.sp,
                            fontWeight = FontWeight.Bold,
                            modifier = Modifier.padding(horizontal = 7.dp, vertical = 4.dp),
                        )
                    }
                    Icon(
                        imageVector = if (expanded) Icons.Rounded.KeyboardArrowDown else Icons.Rounded.KeyboardArrowUp,
                        contentDescription = if (expanded) "Collapse" else "Expand",
                        tint = NovaColors.TextSecondary,
                        modifier = Modifier
                            .size(24.dp)
                            .clip(CircleShape)
                            .clickable(onClick = onToggleExpanded),
                    )
                }
            }

            AnimatedVisibility(visible = expanded) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    // Segmented Tab Row: Transform | Material (or Light/Camera)
                    Surface(
                        color = NovaColors.CanvasBg,
                        shape = RoundedCornerShape(9.dp),
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(modifier = Modifier.padding(3.dp)) {
                            listOf(
                                PortraitCardTab.Transform to "Transform",
                                PortraitCardTab.Material to when (details.kind) {
                                    2 -> "Light"
                                    3 -> "Camera"
                                    else -> "Material"
                                },
                            ).forEach { (tab, label) ->
                                val selected = activeTab == tab
                                Box(
                                    modifier = Modifier
                                        .weight(1f)
                                        .clip(RoundedCornerShape(7.dp))
                                        .background(if (selected) NovaColors.CardBgElevated else Color.Transparent)
                                        .clickable { activeTab = tab }
                                        .padding(vertical = 6.dp),
                                    contentAlignment = Alignment.Center,
                                ) {
                                    Text(
                                        text = label,
                                        color = if (selected) NovaColors.TextPrimary else NovaColors.TextMuted,
                                        fontSize = 12.sp,
                                        fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Medium,
                                    )
                                }
                            }
                        }
                    }

                    if (activeTab == PortraitCardTab.Transform) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Column(
                                modifier = Modifier.weight(1f),
                                verticalArrangement = Arrangement.spacedBy(6.dp),
                            ) {
                                VectorRow(
                                    label = "Pos",
                                    x = details.posX,
                                    y = details.posY,
                                    z = details.posZ,
                                    onEditAxis = { axis ->
                                        editingTransformField = TransformEditTarget("Location", axis, details)
                                    },
                                )
                                VectorRow(
                                    label = "Rot",
                                    x = details.rotX,
                                    y = details.rotY,
                                    z = details.rotZ,
                                    onEditAxis = { axis ->
                                        editingTransformField = TransformEditTarget("Rotation (°)", axis, details)
                                    },
                                )
                                VectorRow(
                                    label = "Scl",
                                    x = details.scaleX,
                                    y = details.scaleY,
                                    z = details.scaleZ,
                                    onEditAxis = { axis ->
                                        editingTransformField = TransformEditTarget("Scale", axis, details)
                                    },
                                )
                            }

                            // Side Quick Action Column matching Reference #1 (Modifiers & Materials)
                            if (details.hasMesh) {
                                Column(
                                    verticalArrangement = Arrangement.spacedBy(6.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                ) {
                                    Surface(
                                        color = NovaColors.CardBg,
                                        shape = RoundedCornerShape(10.dp),
                                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                                        modifier = Modifier
                                            .width(74.dp)
                                            .clip(RoundedCornerShape(10.dp))
                                            .clickable(onClick = onOpenModifiersModal),
                                    ) {
                                        Column(
                                            modifier = Modifier.padding(vertical = 8.dp, horizontal = 4.dp),
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                            verticalArrangement = Arrangement.spacedBy(3.dp),
                                        ) {
                                            Icon(
                                                imageVector = Icons.Rounded.Tune,
                                                contentDescription = "Modifiers",
                                                tint = NovaColors.TextPrimary,
                                                modifier = Modifier.size(16.dp),
                                            )
                                            Text(
                                                text = "Modifiers",
                                                color = NovaColors.TextSecondary,
                                                fontSize = 9.sp,
                                                maxLines = 1,
                                            )
                                        }
                                    }

                                    Surface(
                                        color = NovaColors.CardBg,
                                        shape = RoundedCornerShape(10.dp),
                                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                                        modifier = Modifier
                                            .width(74.dp)
                                            .clip(RoundedCornerShape(10.dp))
                                            .clickable(onClick = onOpenMaterialsModal),
                                    ) {
                                        Column(
                                            modifier = Modifier.padding(vertical = 8.dp, horizontal = 4.dp),
                                            horizontalAlignment = Alignment.CenterHorizontally,
                                            verticalArrangement = Arrangement.spacedBy(3.dp),
                                        ) {
                                            Icon(
                                                imageVector = Icons.Rounded.Palette,
                                                contentDescription = "Materials",
                                                tint = NovaColors.AccentCyan,
                                                modifier = Modifier.size(16.dp),
                                            )
                                            Text(
                                                text = "Materials",
                                                color = NovaColors.TextSecondary,
                                                fontSize = 9.sp,
                                                maxLines = 1,
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        // Material / Light / Camera Quick Tab
                        when (details.kind) {
                            2 -> LightPropertiesCard(controller = controller, details = details, embedInline = true)
                            3 -> CameraPropertiesCard(controller = controller, details = details, embedInline = true)
                            else -> MaterialPropertiesCard(controller = controller, details = details, embedInline = true)
                        }
                    }
                }
            }
        }
    }

    if (renameDialogOpen) {
        RenameNodeDialog(
            initialName = summary.name,
            onDismiss = { renameDialogOpen = false },
            onConfirm = { newName ->
                renameDialogOpen = false
                controller.renameNode(summary.id, newName)
            },
        )
    }

    val target = editingTransformField
    if (target != null) {
        NumericComponentDialog(
            target = target,
            onDismiss = { editingTransformField = null },
            onConfirm = { newVal ->
                editingTransformField = null
                applyTransformFieldUpdate(controller, target, newVal)
            },
        )
    }
}

/**
 * Portrait Floating Edit Mode Bottom Sheet matching Reference #2:
 * Drag handle, Component Name, Pos/Rot telemetry bar, and 2x4 Modeling Tool Grid.
 */
@Composable
fun PortraitEditModeBottomSheet(
    controller: ViewportController,
    expanded: Boolean,
    onToggleExpanded: () -> Unit,
    onOpenAddMenu: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val details = controller.selectedNodeDetails
    val summary = controller.sceneNodes.firstOrNull { it.id == details?.id }
    val es = controller.editState
    var showAdvanced by remember { mutableStateOf(false) }

    Surface(
        color = NovaColors.PanelBg.copy(alpha = 0.96f),
        shape = RoundedCornerShape(18.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        shadowElevation = 12.dp,
        modifier = modifier.fillMaxWidth(),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            // Drag Handle Pill
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable(onClick = onToggleExpanded),
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .width(36.dp)
                        .height(4.dp)
                        .clip(CircleShape)
                        .background(NovaColors.BorderBright),
                )
            }

            // Header row: Component Name + Collapse Arrow
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = summary?.name ?: "Edit Mesh",
                    color = NovaColors.TextPrimary,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    MiniActionChip(
                        label = if (es.activeSelectedCount > 0) "Clear (${es.activeSelectedCount})" else "Select All",
                        active = false,
                        onClick = { controller.editClearOrSelectAll() },
                    )
                    Icon(
                        imageVector = if (expanded) Icons.Rounded.KeyboardArrowDown else Icons.Rounded.KeyboardArrowUp,
                        contentDescription = null,
                        tint = NovaColors.TextSecondary,
                        modifier = Modifier
                            .size(24.dp)
                            .clip(CircleShape)
                            .clickable(onClick = onToggleExpanded),
                    )
                }
            }

            AnimatedVisibility(visible = expanded) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    // Pos / Rot summary pill row
                    if (details != null) {
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(6.dp),
                        ) {
                            Surface(
                                color = NovaColors.CardBg,
                                shape = RoundedCornerShape(7.dp),
                                border = BorderStroke(1.dp, NovaColors.PanelBorder),
                                modifier = Modifier.weight(1f),
                            ) {
                                Text(
                                    text = String.format(
                                        Locale.US,
                                        "Pos: %.2f, %.2f, %.2f",
                                        details.posX, details.posY, details.posZ,
                                    ),
                                    color = NovaColors.TextSecondary,
                                    fontSize = 10.sp,
                                    fontFamily = FontFamily.Monospace,
                                    maxLines = 1,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
                                )
                            }
                            Surface(
                                color = NovaColors.CardBg,
                                shape = RoundedCornerShape(7.dp),
                                border = BorderStroke(1.dp, NovaColors.PanelBorder),
                                modifier = Modifier.weight(0.95f),
                            ) {
                                Text(
                                    text = String.format(
                                        Locale.US,
                                        "Rot: %.0f°, %.0f°, %.0f°",
                                        details.rotX, details.rotY, details.rotZ,
                                    ),
                                    color = NovaColors.TextSecondary,
                                    fontSize = 10.sp,
                                    fontFamily = FontFamily.Monospace,
                                    maxLines = 1,
                                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
                                )
                            }
                        }
                    }

                    // 2x4 Modeling Tool Grid matching Reference #2
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        EditGridButton("Extrude", highlighted = true, Modifier.weight(1f)) { controller.editExtrude() }
                        EditGridButton("Bevel", false, Modifier.weight(1f)) { controller.editBevel() }
                        EditGridButton("Inset", false, Modifier.weight(1f)) { controller.editInset() }
                        EditGridButton("Loop Cut", false, Modifier.weight(1f)) { controller.editLoopCut() }
                    }
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                    ) {
                        EditGridButton("Subdivide", false, Modifier.weight(1f)) { controller.editSubdivide() }
                        EditGridButton("Merge", false, Modifier.weight(1f)) { controller.editMerge() }
                        EditGridButton("+ Add Obj", false, Modifier.weight(1.25f), onClick = onOpenAddMenu)
                        EditGridButton(
                            label = if (showAdvanced) "Less" else "More",
                            highlighted = showAdvanced,
                            modifier = Modifier.weight(0.85f),
                        ) { showAdvanced = !showAdvanced }
                    }

                    AnimatedVisibility(visible = showAdvanced) {
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .horizontalScroll(rememberScrollState()),
                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                        ) {
                            MiniActionChip("Dissolve", false) { controller.editDissolveEdges() }
                            MiniActionChip("Delete", false) { controller.editDelete() }
                            MiniActionChip("Recalc Normals", false) { controller.editRecalculateNormals() }
                            MiniActionChip("Flip Normals", false) { controller.editFlipNormals() }
                            MiniActionChip("Orient", es.showFaceOrientation) { controller.toggleFaceOrientation() }
                            MiniActionChip("Smooth", es.smoothShading) { controller.editSetSmoothShading(true) }
                            MiniActionChip("Flat", !es.smoothShading) { controller.editSetSmoothShading(false) }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun EditGridButton(
    label: String,
    highlighted: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Surface(
        color = if (highlighted) NovaColors.AccentVioletSoft else NovaColors.CardBg,
        shape = RoundedCornerShape(9.dp),
        border = BorderStroke(
            1.dp,
            if (highlighted) NovaColors.AccentViolet else NovaColors.PanelBorder,
        ),
        modifier = modifier
            .height(38.dp)
            .clip(RoundedCornerShape(9.dp))
            .clickable(onClick = onClick),
    ) {
        Box(contentAlignment = Alignment.Center, modifier = Modifier.padding(horizontal = 4.dp)) {
            Text(
                text = label,
                color = if (highlighted) Color.White else NovaColors.TextPrimary,
                fontSize = 12.sp,
                fontWeight = if (highlighted) FontWeight.SemiBold else FontWeight.Medium,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
fun SideInspectorPanel(
    controller: ViewportController,
    modifier: Modifier = Modifier,
) {
    var activeTab by remember { mutableStateOf(InspectorTab.Outliner) }

    Surface(
        color = NovaColors.PanelBg,
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = modifier,
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                InspectorTab.entries.forEach { tab ->
                    val selected = tab == activeTab
                    Surface(
                        color = if (selected) NovaColors.AccentViolet else NovaColors.CardBg,
                        shape = RoundedCornerShape(7.dp),
                        border = BorderStroke(
                            1.dp,
                            if (selected) NovaColors.AccentViolet else NovaColors.PanelBorder,
                        ),
                        modifier = Modifier
                            .weight(1f)
                            .clip(RoundedCornerShape(7.dp))
                            .clickable { activeTab = tab },
                    ) {
                        Text(
                            text = when (tab) {
                                InspectorTab.Outliner -> "Outliner (${controller.sceneNodes.size})"
                                InspectorTab.Properties -> "Props"
                                InspectorTab.World -> "World"
                            },
                            color = if (selected) Color.White else NovaColors.TextSecondary,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.SemiBold,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                            maxLines = 1,
                        )
                    }
                }
            }

            when (activeTab) {
                InspectorTab.Outliner -> OutlinerTabContent(
                    controller = controller,
                    onJumpToProperties = { activeTab = InspectorTab.Properties },
                    modifier = Modifier.weight(1f).fillMaxWidth(),
                )

                InspectorTab.Properties -> PropertiesTabContent(
                    controller = controller,
                    modifier = Modifier.weight(1f).fillMaxWidth(),
                )

                InspectorTab.World -> WorldLightingTabContent(
                    controller = controller,
                    modifier = Modifier.weight(1f).fillMaxWidth(),
                )
            }
        }
    }
}

@Composable
fun OutlinerTabContent(
    controller: ViewportController,
    onJumpToProperties: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val selectedCount = controller.sceneNodes.count { it.selected }
    val primaryNode = controller.sceneNodes.firstOrNull { it.primarySelected }
        ?: controller.sceneNodes.firstOrNull { it.selected }

    var parentMenuOpen by remember { mutableStateOf(false) }

    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            MiniActionChip(
                label = if (controller.multiSelectEnabled) "Multi: ON" else "Multi",
                active = controller.multiSelectEnabled,
                onClick = { controller.toggleMultiSelect() },
            )
            MiniActionChip(
                label = "Group",
                active = false,
                enabled = selectedCount >= 1,
                onClick = { controller.groupSelected() },
            )
            MiniActionChip(
                label = "Unparent",
                active = false,
                enabled = primaryNode != null && primaryNode.parentId != 0L,
                onClick = { controller.unparentSelected() },
            )
            MiniActionChip(
                label = "Dup",
                active = false,
                enabled = selectedCount >= 1,
                onClick = { controller.duplicateSelected() },
            )
            MiniActionChip(
                label = "Del",
                active = false,
                enabled = selectedCount >= 1,
                onClick = { controller.deleteSelected() },
            )
            if (primaryNode != null) {
                Box {
                    MiniActionChip(
                        label = "Parent ▾",
                        active = false,
                        onClick = { parentMenuOpen = true },
                    )
                    DropdownMenu(
                        expanded = parentMenuOpen,
                        onDismissRequest = { parentMenuOpen = false },
                        containerColor = NovaColors.PanelBg,
                    ) {
                        DropdownMenuItem(
                            text = {
                                Text("Scene Root (No Parent)", color = NovaColors.TextPrimary, fontSize = 12.sp)
                            },
                            onClick = {
                                parentMenuOpen = false
                                controller.reparentNode(primaryNode.id, 0L)
                            },
                        )
                        controller.sceneNodes
                            .filter { it.id != primaryNode.id }
                            .forEach { candidate ->
                                DropdownMenuItem(
                                    text = {
                                        Text(
                                            text = "${"  ".repeat(candidate.depth.coerceAtMost(4))}${candidate.name}",
                                            color = NovaColors.TextPrimary,
                                            fontSize = 12.sp,
                                        )
                                    },
                                    onClick = {
                                        parentMenuOpen = false
                                        controller.reparentNode(primaryNode.id, candidate.id)
                                    },
                                )
                            }
                    }
                }
            }
        }

        if (controller.sceneNodes.isEmpty()) {
            Box(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "Scene is empty. Tap + Add to spawn primitives.",
                    color = NovaColors.TextMuted,
                    fontSize = 12.sp,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                items(controller.sceneNodes, key = { it.id }) { node ->
                    OutlinerNodeRow(
                        node = node,
                        onSelect = { controller.selectNode(node.id) },
                        onToggleVisible = { controller.setNodeVisibility(node.id, !node.visible) },
                        onInspect = {
                            controller.selectNode(node.id, additive = false)
                            onJumpToProperties()
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun OutlinerNodeRow(
    node: SceneNodeSummary,
    onSelect: () -> Unit,
    onToggleVisible: () -> Unit,
    onInspect: () -> Unit,
) {
    val bg = when {
        node.primarySelected -> NovaColors.AccentVioletSoft
        node.selected -> NovaColors.AccentVioletSoft.copy(alpha = 0.5f)
        else -> NovaColors.CanvasBg
    }
    val border = when {
        node.primarySelected -> NovaColors.AccentViolet
        node.selected -> NovaColors.AccentViolet.copy(alpha = 0.6f)
        else -> NovaColors.PanelBorder
    }
    val kindIcon = when (node.kind) {
        1 -> Icons.Rounded.ViewInAr
        2 -> Icons.Rounded.LightMode
        3 -> Icons.Rounded.Videocam
        else -> Icons.Rounded.Category
    }
    val badgeColor = when (node.kind) {
        1 -> NovaColors.AccentVioletGlow
        2 -> NovaColors.WarningAmber
        3 -> NovaColors.AccentCyan
        else -> NovaColors.TextMuted
    }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = (node.depth.coerceAtMost(5) * 12).dp)
            .clip(RoundedCornerShape(7.dp))
            .background(bg)
            .border(1.dp, border, RoundedCornerShape(7.dp))
            .clickable(onClick = onSelect)
            .padding(horizontal = 8.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Icon(
            imageVector = kindIcon,
            contentDescription = null,
            tint = badgeColor,
            modifier = Modifier.size(14.dp),
        )

        Text(
            text = node.name,
            color = if (node.visible) NovaColors.TextPrimary else NovaColors.TextMuted,
            fontSize = 11.sp,
            fontWeight = if (node.primarySelected) FontWeight.SemiBold else FontWeight.Normal,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            modifier = Modifier.weight(1f),
        )

        Box(
            modifier = Modifier
                .size(22.dp)
                .clip(RoundedCornerShape(5.dp))
                .clickable(onClick = onToggleVisible),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = if (node.visible) Icons.Rounded.Visibility else Icons.Rounded.VisibilityOff,
                contentDescription = if (node.visible) "Hide" else "Show",
                tint = if (node.visible) NovaColors.TextSecondary else NovaColors.TextMuted,
                modifier = Modifier.size(14.dp),
            )
        }

        if (node.primarySelected) {
            Surface(
                color = NovaColors.AccentViolet.copy(alpha = 0.25f),
                shape = RoundedCornerShape(4.dp),
                modifier = Modifier
                    .clip(RoundedCornerShape(4.dp))
                    .clickable(onClick = onInspect),
            ) {
                Text(
                    text = "Props",
                    color = Color.White,
                    fontSize = 9.sp,
                    modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                )
            }
        }
    }
}

@Composable
private fun PropertiesTabContent(
    controller: ViewportController,
    modifier: Modifier = Modifier,
) {
    val details = controller.selectedNodeDetails
    val summary = controller.sceneNodes.firstOrNull { it.id == details?.id }
    if (details == null || summary == null) {
        Box(
            modifier = modifier.fillMaxSize(),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = "Tap an object in the viewport or Outliner to inspect its properties.",
                color = NovaColors.TextMuted,
                fontSize = 12.sp,
                modifier = Modifier.padding(12.dp),
            )
        }
        return
    }

    var renameDialogOpen by remember(details.id) { mutableStateOf(false) }
    var editingTransformField by remember { mutableStateOf<TransformEditTarget?>(null) }

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Surface(
            color = NovaColors.CardBg,
            shape = RoundedCornerShape(9.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = summary.name,
                        color = NovaColors.TextPrimary,
                        fontSize = 13.sp,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = when (details.kind) {
                            1 -> "Mesh Object"
                            2 -> "Light Node"
                            3 -> "Camera Node"
                            else -> "Empty / Group"
                        },
                        color = NovaColors.TextMuted,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    MiniActionChip("Rename", false) { renameDialogOpen = true }
                    MiniActionChip(if (details.visible) "Visible" else "Hidden", details.visible) {
                        controller.setNodeVisibility(details.id, !details.visible)
                    }
                }
            }
        }

        if (controller.editState.interactionMode == InteractionUiMode.Edit && details.hasMesh) {
            EditModelingPropertiesCard(controller = controller)
        } else {
            Surface(
                color = NovaColors.CardBg,
                shape = RoundedCornerShape(9.dp),
                border = BorderStroke(1.dp, NovaColors.PanelBorder),
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                    verticalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = "Transform",
                            color = NovaColors.TextPrimary,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                            MiniActionChip("Key ◆", false) { controller.keySelectedNodeTrs(7) }
                            if (details.hasMesh) {
                                MiniActionChip("Edit Mesh", false) {
                                    controller.setInteractionMode(InteractionUiMode.Edit)
                                }
                            }
                        }
                    }
                    VectorRow("Loc", details.posX, details.posY, details.posZ) { axis ->
                        editingTransformField = TransformEditTarget("Location", axis, details)
                    }
                    VectorRow("Rot", details.rotX, details.rotY, details.rotZ) { axis ->
                        editingTransformField = TransformEditTarget("Rotation (°)", axis, details)
                    }
                    VectorRow("Scale", details.scaleX, details.scaleY, details.scaleZ) { axis ->
                        editingTransformField = TransformEditTarget("Scale", axis, details)
                    }

                    Row(
                        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        MiniActionChip("Reset TRS", false) {
                            controller.setNodeTransform(details.id, 0f, 0f, 0f, 0f, 0f, 0f, 1f, 1f, 1f)
                        }
                        if (details.hasMesh) {
                            MiniActionChip("Apply Transform", false) { controller.applyTransformSelected() }
                            MiniActionChip("Origin Center", false) { controller.originToGeometrySelected() }
                        }
                    }
                }
            }
        }

        if (details.hasMesh) {
            ModifierStackCard(controller = controller)
            MaterialPropertiesCard(controller = controller, details = details)
        }
        if (details.kind == 2) {
            LightPropertiesCard(controller = controller, details = details)
        }
        if (details.kind == 3) {
            CameraPropertiesCard(controller = controller, details = details)
        }
    }

    if (renameDialogOpen) {
        RenameNodeDialog(
            initialName = summary.name,
            onDismiss = { renameDialogOpen = false },
            onConfirm = { newName ->
                renameDialogOpen = false
                controller.renameNode(summary.id, newName)
            },
        )
    }

    val target = editingTransformField
    if (target != null) {
        NumericComponentDialog(
            target = target,
            onDismiss = { editingTransformField = null },
            onConfirm = { newVal ->
                editingTransformField = null
                applyTransformFieldUpdate(controller, target, newVal)
            },
        )
    }
}

private fun applyTransformFieldUpdate(
    controller: ViewportController,
    target: TransformEditTarget,
    newVal: Float,
) {
    val d = target.details
    var px = d.posX; var py = d.posY; var pz = d.posZ
    var rx = d.rotX; var ry = d.rotY; var rz = d.rotZ
    var sx = d.scaleX; var sy = d.scaleY; var sz = d.scaleZ
    when (target.groupLabel) {
        "Location" -> when (target.axisIndex) {
            0 -> px = newVal; 1 -> py = newVal; else -> pz = newVal
        }
        "Rotation (°)" -> when (target.axisIndex) {
            0 -> rx = newVal; 1 -> ry = newVal; else -> rz = newVal
        }
        else -> {
            val safe = if (kotlin.math.abs(newVal) < 0.001f) 0.001f else newVal
            when (target.axisIndex) {
                0 -> sx = safe; 1 -> sy = safe; else -> sz = safe
            }
        }
    }
    controller.setNodeTransform(d.id, px, py, pz, rx, ry, rz, sx, sy, sz)
}

@Composable
private fun MaterialPropertiesCard(
    controller: ViewportController,
    details: NodeDetails,
    embedInline: Boolean = false,
) {
    var metallic by remember(details.id, details.metallic) { mutableFloatStateOf(details.metallic) }
    var roughness by remember(details.id, details.roughness) { mutableFloatStateOf(details.roughness) }
    var presetMenuOpen by remember { mutableStateOf(false) }
    val assignedMat = controller.materials.firstOrNull { it.id == details.materialId }

    val content: @Composable () -> Unit = {
        Column(
            modifier = Modifier.fillMaxWidth().let { if (embedInline) it else it.padding(8.dp) },
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = assignedMat?.name ?: "Material",
                    color = NovaColors.TextPrimary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    MiniActionChip(
                        label = "Key ◆",
                        active = false,
                        onClick = {
                            val mid = details.materialId.takeIf { it > 0L } ?: controller.selectedMaterialId
                            controller.keyMaterial(mid)
                        },
                    )
                    Box {
                        MiniActionChip(label = "Preset ▾", active = presetMenuOpen) {
                            presetMenuOpen = true
                        }
                        DropdownMenu(
                            expanded = presetMenuOpen,
                            onDismissRequest = { presetMenuOpen = false },
                            containerColor = NovaColors.PanelBg,
                        ) {
                            MaterialUiPreset.entries.forEach { preset ->
                                DropdownMenuItem(
                                    text = { Text(preset.label, color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                    onClick = {
                                        presetMenuOpen = false
                                        controller.createMaterialFromPreset(preset, assignSelected = true)
                                    },
                                )
                            }
                        }
                    }
                }
            }

            if (controller.materials.size > 1) {
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    controller.materials.forEach { mat ->
                        MiniActionChip(
                            label = mat.name,
                            active = mat.id == details.materialId,
                            onClick = { controller.assignMaterialToSelected(mat.id) },
                        )
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                MATERIAL_SWATCHES.forEach { rgba ->
                    val swatchColor = Color(rgba[0], rgba[1], rgba[2], rgba[3])
                    val isCurrent = kotlin.math.abs(details.colorR - rgba[0]) < 0.03f &&
                        kotlin.math.abs(details.colorG - rgba[1]) < 0.03f &&
                        kotlin.math.abs(details.colorB - rgba[2]) < 0.03f
                    Box(
                        modifier = Modifier
                            .size(20.dp)
                            .clip(CircleShape)
                            .background(swatchColor)
                            .border(
                                width = if (isCurrent) 2.dp else 1.dp,
                                color = if (isCurrent) Color.White else NovaColors.PanelBorder,
                                shape = CircleShape,
                            )
                            .clickable {
                                controller.setNodeMaterial(
                                    details.id,
                                    rgba[0], rgba[1], rgba[2], rgba[3],
                                    metallic, roughness,
                                )
                            },
                    )
                }
            }

            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Metallic", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(
                        String.format(Locale.US, "%.2f", metallic),
                        color = NovaColors.TextPrimary,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
                Slider(
                    value = metallic,
                    onValueChange = {
                        metallic = it
                        controller.setNodeMaterial(
                            details.id,
                            details.colorR, details.colorG, details.colorB, details.colorA,
                            metallic, roughness,
                        )
                    },
                    valueRange = 0f..1f,
                    colors = SliderDefaults.colors(
                        thumbColor = NovaColors.AccentViolet,
                        activeTrackColor = NovaColors.AccentViolet,
                        inactiveTrackColor = NovaColors.PanelBorder,
                    ),
                    modifier = Modifier.height(22.dp),
                )
            }

            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Roughness", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(
                        String.format(Locale.US, "%.2f", roughness),
                        color = NovaColors.TextPrimary,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
                Slider(
                    value = roughness,
                    onValueChange = {
                        roughness = it
                        controller.setNodeMaterial(
                            details.id,
                            details.colorR, details.colorG, details.colorB, details.colorA,
                            metallic, roughness,
                        )
                    },
                    valueRange = 0.04f..1f,
                    colors = SliderDefaults.colors(
                        thumbColor = NovaColors.AccentViolet,
                        activeTrackColor = NovaColors.AccentViolet,
                        inactiveTrackColor = NovaColors.PanelBorder,
                    ),
                    modifier = Modifier.height(22.dp),
                )
            }
        }
    }

    if (embedInline) {
        content()
    } else {
        Surface(
            color = NovaColors.CardBg,
            shape = RoundedCornerShape(9.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
        ) {
            content()
        }
    }
}

@Composable
private fun LightPropertiesCard(
    controller: ViewportController,
    details: NodeDetails,
    embedInline: Boolean = false,
) {
    var intensity by remember(details.id, details.lightIntensity) {
        mutableFloatStateOf(details.lightIntensity)
    }
    var lightKind by remember(details.id, details.lightKind) {
        mutableIntStateOf(details.lightKind)
    }
    var spotInner by remember(details.id, details.lightSpotInnerDeg) {
        mutableFloatStateOf(details.lightSpotInnerDeg)
    }
    var spotOuter by remember(details.id, details.lightSpotOuterDeg) {
        mutableFloatStateOf(details.lightSpotOuterDeg)
    }
    var range by remember(details.id, details.lightRange) {
        mutableFloatStateOf(details.lightRange)
    }

    fun pushLight(
        castShadows: Boolean = details.lightCastShadows,
        colR: Float = details.lightColorR,
        colG: Float = details.lightColorG,
        colB: Float = details.lightColorB,
    ) {
        controller.setNodeLightEx(
            details.id, lightKind, colR, colG, colB,
            intensity, castShadows, spotInner, spotOuter, range,
        )
    }

    val content: @Composable () -> Unit = {
        Column(
            modifier = Modifier.fillMaxWidth().let { if (embedInline) it else it.padding(8.dp) },
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    listOf(1 to "Point", 0 to "Sun", 2 to "Spot").forEach { (kind, label) ->
                        MiniActionChip(
                            label = label,
                            active = lightKind == kind,
                            onClick = {
                                lightKind = kind
                                pushLight()
                            },
                        )
                    }
                }
                MiniActionChip("Key ◆", false) { controller.keySelectedLight() }
            }

            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                LIGHT_SWATCHES.forEach { rgb ->
                    Box(
                        modifier = Modifier
                            .size(20.dp)
                            .clip(CircleShape)
                            .background(Color(rgb[0], rgb[1], rgb[2]))
                            .border(1.dp, NovaColors.PanelBorder, CircleShape)
                            .clickable {
                                pushLight(colR = rgb[0], colG = rgb[1], colB = rgb[2])
                            },
                    )
                }
            }

            Column {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                ) {
                    Text("Intensity", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(
                        String.format(Locale.US, "%.0f k", intensity / 1000f),
                        color = NovaColors.TextPrimary,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
                Slider(
                    value = intensity.coerceIn(1000f, 180000f),
                    onValueChange = {
                        intensity = it
                        pushLight()
                    },
                    valueRange = 1000f..180000f,
                    colors = SliderDefaults.colors(
                        thumbColor = NovaColors.WarningAmber,
                        activeTrackColor = NovaColors.WarningAmber,
                        inactiveTrackColor = NovaColors.PanelBorder,
                    ),
                    modifier = Modifier.height(22.dp),
                )
            }

            if (lightKind == 2) {
                Column {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Spot Outer Cone", color = NovaColors.TextSecondary, fontSize = 10.sp)
                        Text(
                            String.format(Locale.US, "%.0f°", spotOuter),
                            color = NovaColors.TextPrimary,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Slider(
                        value = spotOuter.coerceIn(8f, 80f),
                        onValueChange = {
                            spotOuter = it
                            if (spotInner > spotOuter - 2f) {
                                spotInner = (spotOuter - 4f).coerceAtLeast(2f)
                            }
                            pushLight()
                        },
                        valueRange = 8f..80f,
                        colors = SliderDefaults.colors(
                            thumbColor = NovaColors.AccentViolet,
                            activeTrackColor = NovaColors.AccentViolet,
                        ),
                        modifier = Modifier.height(22.dp),
                    )
                }
            }

            if (lightKind != 0) {
                Column {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Range / Falloff", color = NovaColors.TextSecondary, fontSize = 10.sp)
                        Text(
                            String.format(Locale.US, "%.1f m", range),
                            color = NovaColors.TextPrimary,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Slider(
                        value = range.coerceIn(2f, 40f),
                        onValueChange = {
                            range = it
                            pushLight()
                        },
                        valueRange = 2f..40f,
                        colors = SliderDefaults.colors(
                            thumbColor = NovaColors.AccentViolet,
                            activeTrackColor = NovaColors.AccentViolet,
                        ),
                        modifier = Modifier.height(22.dp),
                    )
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Cast Shadows", color = NovaColors.TextSecondary, fontSize = 11.sp)
                Switch(
                    checked = details.lightCastShadows,
                    onCheckedChange = { checked -> pushLight(castShadows = checked) },
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = Color.White,
                        checkedTrackColor = NovaColors.AccentViolet,
                    ),
                )
            }
        }
    }

    if (embedInline) content() else Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(9.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
    ) { content() }
}

@Composable
private fun CameraPropertiesCard(
    controller: ViewportController,
    details: NodeDetails,
    embedInline: Boolean = false,
) {
    var fovDeg by remember(details.id, details.cameraFovDeg) {
        mutableFloatStateOf(details.cameraFovDeg)
    }
    val isActiveCam = controller.timeline.activeCameraNodeId == details.id

    val content: @Composable () -> Unit = {
        Column(
            modifier = Modifier.fillMaxWidth().let { if (embedInline) it else it.padding(8.dp) },
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                MiniActionChip(
                    label = if (isActiveCam) "Active Cam View ✓" else "Look Through Cam",
                    active = isActiveCam,
                    onClick = {
                        controller.setActiveCameraNode(if (isActiveCam) 0L else details.id)
                    },
                )
                MiniActionChip("Key ◆", false) { controller.keySelectedCamera() }
            }

            Text(
                text = String.format(Locale.US, "Field of View: %.1f°", fovDeg),
                color = NovaColors.TextSecondary,
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
            )
            Slider(
                value = fovDeg,
                onValueChange = {
                    fovDeg = it
                    controller.setCameraNodeFov(details.id, it)
                },
                valueRange = 20f..100f,
                colors = SliderDefaults.colors(
                    thumbColor = NovaColors.AccentViolet,
                    activeTrackColor = NovaColors.AccentViolet,
                ),
                modifier = Modifier.height(22.dp),
            )

            Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                listOf(24f to "24° Wide", 50f to "50° Std", 85f to "85° Tele").forEach { (preset, label) ->
                    MiniActionChip(
                        label = label,
                        active = kotlin.math.abs(fovDeg - preset) < 1.5f,
                        onClick = {
                            fovDeg = preset
                            controller.setCameraNodeFov(details.id, preset)
                        },
                    )
                }
            }
        }
    }

    if (embedInline) content() else Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(9.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
    ) { content() }
}

@Composable
fun WorldLightingTabContent(
    controller: ViewportController,
    embedInline: Boolean = false,
    modifier: Modifier = Modifier,
) {
    val env = controller.environment

    Column(
        modifier = if (embedInline) modifier else modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (controller.missingTextureWarnings.isNotEmpty()) {
            Surface(
                color = NovaColors.WarningAmber.copy(alpha = 0.16f),
                shape = RoundedCornerShape(8.dp),
                border = BorderStroke(1.dp, NovaColors.WarningAmber.copy(alpha = 0.65f)),
            ) {
                Text(
                    text = controller.missingTextureWarnings.joinToString("\n"),
                    color = NovaColors.WarningAmber,
                    fontSize = 11.sp,
                    modifier = Modifier.fillMaxWidth().padding(8.dp),
                )
            }
        }

        Surface(
            color = if (embedInline) NovaColors.CanvasBg else NovaColors.CardBg,
            shape = RoundedCornerShape(8.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
        ) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    text = "Studio Lighting Rigs",
                    color = NovaColors.TextPrimary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    LightingRigPreset.entries.chunked(2).forEach { pair ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            pair.forEach { rig ->
                                Box(modifier = Modifier.weight(1f)) {
                                    MiniActionChip(
                                        label = rig.label,
                                        active = false,
                                        onClick = { controller.applyLightingRig(rig) },
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        Surface(
            color = if (embedInline) NovaColors.CanvasBg else NovaColors.CardBg,
            shape = RoundedCornerShape(8.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
        ) {
            Column(
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    text = "HDR Environment IBL",
                    color = NovaColors.TextPrimary,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    EnvironmentUiPreset.entries.chunked(2).forEach { pair ->
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            pair.forEach { preset ->
                                Box(modifier = Modifier.weight(1f)) {
                                    MiniActionChip(
                                        label = preset.label,
                                        active = env.preset == preset.code,
                                        onClick = {
                                            controller.updateEnvironment(env.copy(preset = preset.code))
                                        },
                                    )
                                }
                            }
                        }
                    }
                }

                Column {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Env Rotation", color = NovaColors.TextSecondary, fontSize = 10.sp)
                        Text(
                            String.format(Locale.US, "%.0f°", env.rotationDeg),
                            color = NovaColors.TextPrimary,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Slider(
                        value = env.rotationDeg.coerceIn(0f, 360f),
                        onValueChange = { controller.updateEnvironment(env.copy(rotationDeg = it)) },
                        valueRange = 0f..360f,
                        colors = SliderDefaults.colors(
                            thumbColor = NovaColors.AccentViolet,
                            activeTrackColor = NovaColors.AccentViolet,
                        ),
                        modifier = Modifier.height(22.dp),
                    )
                }

                Column {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("IBL Strength", color = NovaColors.TextSecondary, fontSize = 10.sp)
                        Text(
                            String.format(Locale.US, "%.2fx", env.intensity),
                            color = NovaColors.TextPrimary,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Slider(
                        value = env.intensity.coerceIn(0.1f, 2.5f),
                        onValueChange = { controller.updateEnvironment(env.copy(intensity = it)) },
                        valueRange = 0.1f..2.5f,
                        colors = SliderDefaults.colors(
                            thumbColor = NovaColors.AccentViolet,
                            activeTrackColor = NovaColors.AccentViolet,
                        ),
                        modifier = Modifier.height(22.dp),
                    )
                }

                Column {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween,
                    ) {
                        Text("Exposure", color = NovaColors.TextSecondary, fontSize = 10.sp)
                        Text(
                            String.format(Locale.US, "%.2f", env.exposure),
                            color = NovaColors.TextPrimary,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                    Slider(
                        value = env.exposure.coerceIn(0.4f, 2.2f),
                        onValueChange = { controller.updateEnvironment(env.copy(exposure = it)) },
                        valueRange = 0.4f..2.2f,
                        colors = SliderDefaults.colors(
                            thumbColor = NovaColors.AccentViolet,
                            activeTrackColor = NovaColors.AccentViolet,
                        ),
                        modifier = Modifier.height(22.dp),
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("Show HDR Skybox", color = NovaColors.TextSecondary, fontSize = 11.sp)
                    Switch(
                        checked = env.showSkybox,
                        onCheckedChange = { checked ->
                            controller.updateEnvironment(env.copy(showSkybox = checked))
                            if (checked && controller.shadingMode.code < 2) {
                                controller.setShadingMode(ShadingUiMode.RenderPreview)
                            }
                        },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = Color.White,
                            checkedTrackColor = NovaColors.AccentViolet,
                        ),
                    )
                }

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text("Bloom Glow", color = NovaColors.TextSecondary, fontSize = 11.sp)
                    Switch(
                        checked = env.bloomEnabled,
                        onCheckedChange = { checked ->
                            controller.updateEnvironment(env.copy(bloomEnabled = checked))
                        },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = Color.White,
                            checkedTrackColor = NovaColors.AccentViolet,
                        ),
                    )
                }
            }
        }
    }
}

@Composable
private fun VectorRow(
    label: String,
    x: Float,
    y: Float,
    z: Float,
    onEditAxis: (Int) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Text(
            text = label,
            color = NovaColors.TextSecondary,
            fontSize = 10.sp,
            fontFamily = FontFamily.Monospace,
            modifier = Modifier.width(28.dp),
        )
        AxisPill("X", NovaColors.AxisX, x, Modifier.weight(1f)) { onEditAxis(0) }
        AxisPill("Y", NovaColors.AxisY, y, Modifier.weight(1f)) { onEditAxis(1) }
        AxisPill("Z", NovaColors.AxisZ, z, Modifier.weight(1f)) { onEditAxis(2) }
    }
}

@Composable
private fun AxisPill(
    axis: String,
    accent: Color,
    value: Float,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Surface(
        color = NovaColors.CanvasBg,
        shape = RoundedCornerShape(6.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = modifier
            .clip(RoundedCornerShape(6.dp))
            .clickable(onClick = onClick),
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 5.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(3.dp),
        ) {
            Text(
                text = axis,
                color = accent,
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                fontFamily = FontFamily.Monospace,
            )
            Text(
                text = String.format(Locale.US, "%.2f", value),
                color = NovaColors.TextPrimary,
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
                maxLines = 1,
            )
        }
    }
}

@Composable
fun MiniActionChip(
    label: String,
    active: Boolean,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Surface(
        color = when {
            !enabled -> NovaColors.CanvasBg.copy(alpha = 0.5f)
            active -> NovaColors.AccentVioletSoft
            else -> NovaColors.CanvasBg
        },
        shape = RoundedCornerShape(6.dp),
        border = BorderStroke(
            1.dp,
            when {
                !enabled -> NovaColors.PanelBorder.copy(alpha = 0.4f)
                active -> NovaColors.AccentViolet
                else -> NovaColors.PanelBorder
            },
        ),
        modifier = Modifier
            .clip(RoundedCornerShape(6.dp))
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        Text(
            text = label,
            color = when {
                !enabled -> NovaColors.TextMuted.copy(alpha = 0.5f)
                active -> Color.White
                else -> NovaColors.TextSecondary
            },
            fontSize = 10.sp,
            fontWeight = if (active) FontWeight.SemiBold else FontWeight.Medium,
            maxLines = 1,
            modifier = Modifier.padding(horizontal = 7.dp, vertical = 4.dp),
        )
    }
}

private data class TransformEditTarget(
    val groupLabel: String,
    val axisIndex: Int,
    val details: NodeDetails,
) {
    val axisLabel: String
        get() = when (axisIndex) {
            0 -> "X"
            1 -> "Y"
            else -> "Z"
        }

    val currentValue: Float
        get() = when (groupLabel) {
            "Location" -> when (axisIndex) {
                0 -> details.posX; 1 -> details.posY; else -> details.posZ
            }
            "Rotation (°)" -> when (axisIndex) {
                0 -> details.rotX; 1 -> details.rotY; else -> details.rotZ
            }
            else -> when (axisIndex) {
                0 -> details.scaleX; 1 -> details.scaleY; else -> details.scaleZ
            }
        }
}

@Composable
private fun RenameNodeDialog(
    initialName: String,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    var text by remember(initialName) { mutableStateOf(initialName) }
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
        title = { Text("Rename Object", color = NovaColors.TextPrimary, fontSize = 16.sp) },
        text = {
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                singleLine = true,
                label = { Text("Object Name") },
            )
        },
        confirmButton = {
            TextButton(
                onClick = { if (text.isNotBlank()) onConfirm(text.trim()) },
            ) {
                Text("Save", color = NovaColors.AccentViolet)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel", color = NovaColors.TextSecondary)
            }
        },
    )
}

@Composable
private fun NumericComponentDialog(
    target: TransformEditTarget,
    onDismiss: () -> Unit,
    onConfirm: (Float) -> Unit,
) {
    var text by remember(target) {
        mutableStateOf(String.format(Locale.US, "%.3f", target.currentValue))
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
        title = {
            Text(
                text = "Edit ${target.groupLabel} ${target.axisLabel}",
                color = NovaColors.TextPrimary,
                fontSize = 16.sp,
            )
        },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(
                    value = text,
                    onValueChange = { text = it },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    label = { Text("${target.axisLabel} Value") },
                )
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(-1f, -0.5f, 0f, 0.5f, 1f).forEach { nudge ->
                        MiniActionChip(
                            label = if (nudge == 0f) "=0" else if (nudge > 0f) "+$nudge" else "$nudge",
                            active = false,
                            onClick = {
                                val base = text.toFloatOrNull() ?: target.currentValue
                                val next = if (nudge == 0f) 0f else base + nudge
                                text = String.format(Locale.US, "%.3f", next)
                            },
                        )
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = {
                    val parsed = text.toFloatOrNull()
                    if (parsed != null && parsed.isFinite()) {
                        onConfirm(parsed)
                    }
                },
            ) {
                Text("Apply", color = NovaColors.AccentViolet)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) {
                Text("Cancel", color = NovaColors.TextSecondary)
            }
        },
    )
}

@Composable
private fun EditModelingPropertiesCard(
    controller: ViewportController,
    embedInline: Boolean = false,
) {
    val es = controller.editState
    var extrudeDist by remember { mutableFloatStateOf(0.35f) }
    var insetFactor by remember { mutableFloatStateOf(0.25f) }
    var bevelWidth by remember { mutableFloatStateOf(0.12f) }

    val content: @Composable () -> Unit = {
        Column(
            modifier = Modifier.fillMaxWidth().let { if (embedInline) it else it.padding(8.dp) },
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "V:${es.totalVerts} E:${es.totalEdges} F:${es.totalFaces} · Sel:${es.activeSelectedCount}",
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                )
                MiniActionChip("Object Mode", false) {
                    controller.setInteractionMode(InteractionUiMode.Object)
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                EditSelectUiMode.entries.forEach { sm ->
                    MiniActionChip(sm.label, es.selectMode == sm) { controller.setEditSelectMode(sm) }
                }
                MiniActionChip(if (es.activeSelectedCount > 0) "Clear Sel" else "Select All", false) {
                    controller.editClearOrSelectAll()
                }
            }

            Column {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("Extrude Distance", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(String.format(Locale.US, "%.2f", extrudeDist), color = NovaColors.TextPrimary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                }
                Slider(
                    value = extrudeDist,
                    onValueChange = { extrudeDist = it },
                    valueRange = -1.0f..1.5f,
                    colors = SliderDefaults.colors(thumbColor = NovaColors.AccentViolet, activeTrackColor = NovaColors.AccentViolet),
                    modifier = Modifier.height(22.dp),
                )
            }

            Column {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("Inset Factor", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(String.format(Locale.US, "%.2f", insetFactor), color = NovaColors.TextPrimary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                }
                Slider(
                    value = insetFactor,
                    onValueChange = { insetFactor = it },
                    valueRange = 0.05f..0.85f,
                    colors = SliderDefaults.colors(thumbColor = NovaColors.AccentViolet, activeTrackColor = NovaColors.AccentViolet),
                    modifier = Modifier.height(22.dp),
                )
            }

            Column {
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Text("Bevel Width", color = NovaColors.TextSecondary, fontSize = 10.sp)
                    Text(String.format(Locale.US, "%.2f", bevelWidth), color = NovaColors.TextPrimary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                }
                Slider(
                    value = bevelWidth,
                    onValueChange = { bevelWidth = it },
                    valueRange = 0.02f..0.45f,
                    colors = SliderDefaults.colors(thumbColor = NovaColors.AccentViolet, activeTrackColor = NovaColors.AccentViolet),
                    modifier = Modifier.height(22.dp),
                )
            }

            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                MiniActionChip("Extrude", false) { controller.editExtrude(extrudeDist) }
                MiniActionChip("Inset", false) { controller.editInset(insetFactor) }
                MiniActionChip("Bevel", false) { controller.editBevel(bevelWidth) }
                MiniActionChip("Loop Cut", false) { controller.editLoopCut() }
            }
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                MiniActionChip("Subdivide", false) { controller.editSubdivide() }
                MiniActionChip("Merge", false) { controller.editMerge() }
                MiniActionChip("Dissolve", false) { controller.editDissolveEdges() }
                MiniActionChip("Delete", false) { controller.editDelete() }
            }
            Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                MiniActionChip("Recalc", false) { controller.editRecalculateNormals() }
                MiniActionChip("Flip", false) { controller.editFlipNormals() }
                MiniActionChip("Orient", es.showFaceOrientation) { controller.toggleFaceOrientation() }
                MiniActionChip("Smooth", es.smoothShading) { controller.editSetSmoothShading(true) }
                MiniActionChip("Flat", !es.smoothShading) { controller.editSetSmoothShading(false) }
            }
        }
    }

    if (embedInline) content() else Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(9.dp),
        border = BorderStroke(1.dp, NovaColors.AccentViolet.copy(alpha = 0.55f)),
    ) { content() }
}

@Composable
fun ModifierStackCard(
    controller: ViewportController,
    embedInline: Boolean = false,
) {
    val mods = controller.modifiers
    val es = controller.editState
    val content: @Composable () -> Unit = {
        Column(
            modifier = Modifier.fillMaxWidth().let { if (embedInline) it else it.padding(8.dp) },
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    MiniActionChip(
                        label = if (es.smoothShading) "Smooth" else "Flat",
                        active = es.smoothShading,
                        onClick = { controller.editSetSmoothShading(!es.smoothShading) },
                    )
                    if (mods.isNotEmpty()) {
                        MiniActionChip("Apply All", false) { controller.applyModifierStack() }
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                ModifierUiKind.entries.forEach { kind ->
                    MiniActionChip("+ ${kind.label}", false) { controller.addModifier(kind) }
                }
            }

            mods.forEach { spec ->
                ModifierRowItem(
                    spec = spec,
                    canMoveUp = spec.index > 0,
                    canMoveDown = spec.index + 1 < mods.size,
                    onUpdate = { controller.updateModifier(it) },
                    onMove = { delta -> controller.moveModifier(spec.index, delta) },
                    onRemove = { controller.removeModifier(spec.index) },
                )
            }
        }
    }

    if (embedInline) content() else Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(9.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
    ) { content() }
}

@Composable
private fun ModifierRowItem(
    spec: ModifierUiSpec,
    canMoveUp: Boolean,
    canMoveDown: Boolean,
    onUpdate: (ModifierUiSpec) -> Unit,
    onMove: (Int) -> Unit,
    onRemove: () -> Unit,
) {
    Surface(
        color = NovaColors.CanvasBg,
        shape = RoundedCornerShape(7.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(6.dp),
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = spec.kind.label,
                    color = if (spec.enabled) NovaColors.TextPrimary else NovaColors.TextMuted,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                    MiniActionChip(if (spec.enabled) "On" else "Off", spec.enabled) {
                        onUpdate(spec.copy(enabled = !spec.enabled))
                    }
                    if (canMoveUp) MiniActionChip("Up", false) { onMove(-1) }
                    if (canMoveDown) MiniActionChip("Dn", false) { onMove(1) }
                    MiniActionChip("Del", false, onClick = onRemove)
                }
            }

            when (spec.kind) {
                ModifierUiKind.Mirror -> {
                    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        listOf(0 to "X", 1 to "Y", 2 to "Z").forEach { (ax, lbl) ->
                            MiniActionChip("Axis $lbl", spec.axis == ax) { onUpdate(spec.copy(axis = ax)) }
                        }
                        MiniActionChip("Merge", spec.merge) { onUpdate(spec.copy(merge = !spec.merge)) }
                    }
                }
                ModifierUiKind.Subdivision -> {
                    Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        listOf(1, 2, 3).forEach { lvl ->
                            MiniActionChip("Lvl $lvl", spec.levels == lvl) { onUpdate(spec.copy(levels = lvl)) }
                        }
                    }
                }
                ModifierUiKind.Solidify -> {
                    Column {
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                            Text("Thickness", color = NovaColors.TextSecondary, fontSize = 10.sp)
                            Text(String.format(Locale.US, "%.2f", spec.thickness), color = NovaColors.TextPrimary, fontSize = 10.sp, fontFamily = FontFamily.Monospace)
                        }
                        Slider(
                            value = spec.thickness,
                            onValueChange = { onUpdate(spec.copy(thickness = it)) },
                            valueRange = -0.4f..0.4f,
                            colors = SliderDefaults.colors(thumbColor = NovaColors.AccentViolet, activeTrackColor = NovaColors.AccentViolet),
                            modifier = Modifier.height(22.dp),
                        )
                    }
                }
                ModifierUiKind.Array -> {
                    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        listOf(2, 3, 4, 6).forEach { cnt ->
                            MiniActionChip("${cnt}x", spec.count == cnt) { onUpdate(spec.copy(count = cnt)) }
                        }
                        listOf(1.2f, 1.8f, 2.5f).forEach { off ->
                            MiniActionChip("dx $off", kotlin.math.abs(spec.offsetX - off) < 0.05f) {
                                onUpdate(spec.copy(offsetX = off))
                            }
                        }
                    }
                }
            }
        }
    }
}
