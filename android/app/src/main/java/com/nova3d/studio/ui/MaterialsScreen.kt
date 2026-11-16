package com.nova3d.studio.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.MaterialUiPreset
import com.nova3d.studio.viewport.MaterialUiSpec
import com.nova3d.studio.viewport.ShadingUiMode
import com.nova3d.studio.viewport.TextureUiSpec
import com.nova3d.studio.viewport.ViewportController
import com.nova3d.studio.viewport.ViewportSurface
import java.util.Locale
import kotlin.math.PI
import kotlinx.coroutines.launch

private val MAT_COLOR_SWATCHES = listOf(
    floatArrayOf(0.84f, 0.86f, 0.90f, 1.0f),
    floatArrayOf(0.28f, 0.56f, 0.92f, 1.0f),
    floatArrayOf(0.95f, 0.76f, 0.30f, 1.0f),
    floatArrayOf(0.85f, 0.24f, 0.22f, 1.0f),
    floatArrayOf(0.22f, 0.78f, 0.48f, 1.0f),
    floatArrayOf(0.16f, 0.78f, 0.88f, 1.0f),
    floatArrayOf(0.76f, 0.42f, 0.95f, 1.0f),
    floatArrayOf(0.16f, 0.18f, 0.22f, 1.0f),
)

private val EMISSIVE_SWATCHES = listOf(
    floatArrayOf(0.0f, 0.0f, 0.0f),
    floatArrayOf(0.15f, 0.82f, 1.0f),
    floatArrayOf(1.0f, 0.48f, 0.12f),
    floatArrayOf(0.32f, 1.0f, 0.48f),
    floatArrayOf(0.95f, 0.25f, 0.85f),
    floatArrayOf(1.0f, 0.96f, 0.82f),
)

@Composable
fun MaterialsScreen(
    controller: ViewportController,
    isLifecycleActive: Boolean,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        controller.refreshLookDev()
        controller.setMaterialPreviewSphere(true)
    }

    DisposableEffect(Unit) {
        onDispose {
            controller.setMaterialPreviewSphere(false)
        }
    }

    val imagePickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.GetContent(),
    ) { uri: Uri? ->
        if (uri != null) {
            scope.launch {
                controller.importImageTextureAsync(context, uri, attachAsAlbedoToSelectedMat = true)
            }
        }
    }

    val selectedMat = controller.materials.firstOrNull { it.id == controller.selectedMaterialId }
        ?: controller.materials.firstOrNull()
    var renameTarget by remember { mutableStateOf<MaterialUiSpec?>(null) }

    Column(
        modifier = modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg),
    ) {
        if (controller.missingTextureWarnings.isNotEmpty()) {
            Surface(
                color = NovaColors.WarningAmber.copy(alpha = 0.16f),
                border = BorderStroke(1.dp, NovaColors.WarningAmber.copy(alpha = 0.65f)),
                modifier = Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 4.dp),
                shape = RoundedCornerShape(8.dp),
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 6.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = controller.missingTextureWarnings.joinToString(" | "),
                        color = NovaColors.WarningAmber,
                        fontSize = 11.sp,
                        fontWeight = FontWeight.Medium,
                        modifier = Modifier.weight(1f),
                        maxLines = 2,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Spacer(Modifier.width(8.dp))
                    MatChip(
                        label = "Restore All",
                        active = true,
                        onClick = {
                            controller.textures.filter { it.isMissing }.forEach {
                                controller.markTextureMissing(it.id, false)
                            }
                        },
                    )
                }
            }
        }

        BoxWithConstraints(modifier = Modifier.weight(1f).fillMaxWidth()) {
            val isWide = maxWidth >= 680.dp || (maxWidth > maxHeight && maxWidth >= 520.dp)
            if (isWide) {
                Row(modifier = Modifier.fillMaxSize()) {
                    MaterialPreviewPane(
                        controller = controller,
                        isLifecycleActive = isLifecycleActive,
                        selectedMat = selectedMat,
                        modifier = Modifier.weight(0.45f).fillMaxHeight(),
                    )
                    MaterialEditorColumn(
                        controller = controller,
                        selectedMat = selectedMat,
                        onPickImage = { imagePickerLauncher.launch("image/*") },
                        onRenameRequest = { renameTarget = it },
                        modifier = Modifier.weight(0.55f).fillMaxHeight(),
                    )
                }
            } else {
                Column(modifier = Modifier.fillMaxSize()) {
                    MaterialPreviewPane(
                        controller = controller,
                        isLifecycleActive = isLifecycleActive,
                        selectedMat = selectedMat,
                        modifier = Modifier.fillMaxWidth().height(230.dp),
                    )
                    MaterialEditorColumn(
                        controller = controller,
                        selectedMat = selectedMat,
                        onPickImage = { imagePickerLauncher.launch("image/*") },
                        onRenameRequest = { renameTarget = it },
                        modifier = Modifier.weight(1f).fillMaxWidth(),
                    )
                }
            }
        }
    }

    val ren = renameTarget
    if (ren != null) {
        var text by remember(ren.id) { mutableStateOf(ren.name) }
        AlertDialog(
            onDismissRequest = { renameTarget = null },
            containerColor = NovaColors.PanelBg,
            title = { Text("Rename Material", color = NovaColors.TextPrimary, fontSize = 15.sp) },
            text = {
                OutlinedTextField(
                    value = text,
                    onValueChange = { text = it },
                    singleLine = true,
                    label = { Text("Material Name") },
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        renameTarget = null
                        controller.renameMaterial(ren.id, text)
                    },
                ) { Text("Save", color = NovaColors.AccentBlue) }
            },
            dismissButton = {
                TextButton(onClick = { renameTarget = null }) {
                    Text("Cancel", color = NovaColors.TextSecondary)
                }
            },
        )
    }
}

@Composable
private fun MaterialPreviewPane(
    controller: ViewportController,
    isLifecycleActive: Boolean,
    selectedMat: MaterialUiSpec?,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier
            .padding(8.dp)
            .clip(RoundedCornerShape(10.dp))
            .border(1.dp, NovaColors.PanelBorder, RoundedCornerShape(10.dp))
            .background(NovaColors.CanvasBg),
    ) {
        ViewportSurface(
            controller = controller,
            isActive = isLifecycleActive,
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(Unit) {
                    detectTransformGestures { _, pan, zoom, _ ->
                        val minDim = size.width.coerceAtLeast(1).coerceAtMost(size.height.coerceAtLeast(1)).toFloat()
                        if (pan.x != 0f || pan.y != 0f) {
                            controller.orbit(
                                deltaYawRad = -(pan.x / minDim) * (1.8f * PI.toFloat()),
                                deltaPitchRad = (pan.y / minDim) * (1.2f * PI.toFloat()),
                            )
                        }
                        if (zoom != 1f) {
                            controller.zoom(zoom)
                        }
                    }
                },
        )

        Row(
            modifier = Modifier
                .align(Alignment.TopStart)
                .fillMaxWidth()
                .padding(8.dp)
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            MatChip(
                label = "Sphere",
                active = controller.environment.previewSphere,
                onClick = { controller.setMaterialPreviewSphere(true) },
            )
            MatChip(
                label = "Scene",
                active = !controller.environment.previewSphere,
                onClick = { controller.setMaterialPreviewSphere(false) },
            )
            Spacer(Modifier.width(4.dp))
            listOf(ShadingUiMode.MaterialPreview, ShadingUiMode.RenderPreview).forEach { mode ->
                MatChip(
                    label = mode.label,
                    active = controller.shadingMode == mode,
                    onClick = { controller.setShadingMode(mode) },
                )
            }
        }

        if (selectedMat != null) {
            Surface(
                color = NovaColors.PanelBg.copy(alpha = 0.85f),
                shape = RoundedCornerShape(6.dp),
                modifier = Modifier.align(Alignment.BottomStart).padding(8.dp),
            ) {
                Text(
                    text = "${selectedMat.name} • Orbit to inspect reflections",
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                )
            }
        }
    }
}

@Composable
private fun MaterialEditorColumn(
    controller: ViewportController,
    selectedMat: MaterialUiSpec?,
    onPickImage: () -> Unit,
    onRenameRequest: (MaterialUiSpec) -> Unit,
    modifier: Modifier = Modifier,
) {
    var presetMenuOpen by remember { mutableStateOf(false) }
    val selectedNode = controller.sceneNodes.firstOrNull { it.id == controller.selectedNodeDetails?.id }

    Column(
        modifier = modifier
            .padding(end = 8.dp, bottom = 8.dp, start = 8.dp, top = 4.dp)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        // 1. Material Library
        MatCard(title = "Material Library (${controller.materials.size})") {
            Row(
                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                controller.materials.forEach { mat ->
                    val active = mat.id == selectedMat?.id
                    Surface(
                        color = if (active) NovaColors.AccentBlue.copy(alpha = 0.22f) else NovaColors.PanelBg,
                        shape = RoundedCornerShape(6.dp),
                        border = BorderStroke(
                            1.dp,
                            if (active) NovaColors.AccentBlue else NovaColors.PanelBorder,
                        ),
                        modifier = Modifier.clip(RoundedCornerShape(6.dp)).clickable {
                            controller.selectMaterial(mat.id)
                        },
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(10.dp)
                                    .clip(CircleShape)
                                    .background(Color(mat.r, mat.g, mat.b, 1f))
                                    .border(0.5.dp, Color.White.copy(alpha = 0.6f), CircleShape),
                            )
                            Text(
                                text = mat.name,
                                color = if (active) NovaColors.TextPrimary else NovaColors.TextSecondary,
                                fontSize = 11.sp,
                                fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal,
                            )
                        }
                    }
                }
            }

            Row(
                modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                Box {
                    MatChip(label = "+ Preset ▾", active = presetMenuOpen) {
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

                if (selectedMat != null) {
                    MatChip(
                        label = if (selectedNode != null) "Assign → ${selectedNode.name}" else "Assign Selected",
                        active = selectedNode != null,
                        onClick = { controller.assignMaterialToSelected(selectedMat.id) },
                    )
                    MatChip(label = "Duplicate", active = false) {
                        controller.duplicateMaterial(selectedMat.id)
                    }
                    MatChip(label = "Rename", active = false) {
                        onRenameRequest(selectedMat)
                    }
                    if (controller.materials.size > 1 && selectedMat.id > 1L) {
                        MatChip(label = "Delete", active = false) {
                            controller.deleteMaterial(selectedMat.id)
                        }
                    }
                }
            }
        }

        if (selectedMat != null) {
            // 2. PBR Parameters & Alpha
            MatCard(title = "PBR Surface — ${selectedMat.name}") {
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    MAT_COLOR_SWATCHES.forEach { sw ->
                        Box(
                            modifier = Modifier
                                .size(22.dp)
                                .clip(CircleShape)
                                .background(Color(sw[0], sw[1], sw[2], 1f))
                                .border(1.dp, NovaColors.PanelBorder, CircleShape)
                                .clickable {
                                    controller.updateMaterial(
                                        selectedMat.copy(r = sw[0], g = sw[1], b = sw[2]),
                                    )
                                },
                        )
                    }
                    Spacer(Modifier.width(4.dp))
                    MatChip(
                        label = if (selectedMat.alphaMode == 1) "Alpha: Blend" else "Alpha: Opaque",
                        active = selectedMat.alphaMode == 1,
                        onClick = {
                            val nextMode = if (selectedMat.alphaMode == 1) 0 else 1
                            val nextA = if (nextMode == 1 && selectedMat.a > 0.95f) 0.48f else if (nextMode == 0) 1.0f else selectedMat.a
                            controller.updateMaterial(selectedMat.copy(alphaMode = nextMode, a = nextA))
                        },
                    )
                }

                MatSlider("Opacity", selectedMat.a, 0.08f..1.0f) { v ->
                    controller.updateMaterial(
                        selectedMat.copy(a = v, alphaMode = if (v < 0.995f) 1 else 0),
                    )
                }
                MatSlider("Metallic", selectedMat.metallic, 0f..1f) { v ->
                    controller.updateMaterial(selectedMat.copy(metallic = v))
                }
                MatSlider("Roughness", selectedMat.roughness, 0.04f..1f) { v ->
                    controller.updateMaterial(selectedMat.copy(roughness = v))
                }
                MatSlider("AO Strength", selectedMat.aoStrength, 0f..1f) { v ->
                    controller.updateMaterial(selectedMat.copy(aoStrength = v))
                }
            }

            // 3. Emission & UV Transform
            MatCard(title = "Emission & UV Mapping") {
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    EMISSIVE_SWATCHES.forEach { sw ->
                        Box(
                            modifier = Modifier
                                .size(20.dp)
                                .clip(CircleShape)
                                .background(Color(sw[0], sw[1], sw[2]))
                                .border(1.dp, NovaColors.PanelBorder, CircleShape)
                                .clickable {
                                    val boost = if (selectedMat.emIntensity < 0.2f && (sw[0] + sw[1] + sw[2]) > 0.1f) 1.8f else selectedMat.emIntensity
                                    controller.updateMaterial(
                                        selectedMat.copy(emR = sw[0], emG = sw[1], emB = sw[2], emIntensity = boost),
                                    )
                                },
                        )
                    }
                }
                MatSlider("Emissive Intensity", selectedMat.emIntensity, 0f..5f) { v ->
                    controller.updateMaterial(selectedMat.copy(emIntensity = v))
                }
                MatSlider("UV Tile Scale", selectedMat.uvScaleX, 0.25f..6.0f) { v ->
                    controller.updateMaterial(selectedMat.copy(uvScaleX = v, uvScaleY = v))
                }
            }

            // 4. Texture Map Slots + Image Import + Missing Texture Resilience (MT-2, MT-5)
            MatCard(title = "Texture Maps (${controller.textures.size} assets)") {
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                ) {
                    MatChip(label = "+ Import Image", active = true, onClick = onPickImage)
                    MatChip(label = "Test Missing Tex", active = false) {
                        val tid = controller.registerMissingTestTexture(
                            "Missing Plaster",
                            "device://textures/plaster_missing.png",
                        )
                        if (tid > 0L) {
                            controller.updateMaterial(selectedMat.copy(albedoTexId = tid))
                        }
                    }
                }

                TextureSlotSelector(
                    label = "Albedo Map",
                    selectedTexId = selectedMat.albedoTexId,
                    textures = controller.textures,
                    onSelect = { controller.updateMaterial(selectedMat.copy(albedoTexId = it)) },
                )
                TextureSlotSelector(
                    label = "Normal Map",
                    selectedTexId = selectedMat.normalTexId,
                    textures = controller.textures,
                    onSelect = { controller.updateMaterial(selectedMat.copy(normalTexId = it)) },
                )
                TextureSlotSelector(
                    label = "Roughness Map",
                    selectedTexId = selectedMat.roughnessTexId,
                    textures = controller.textures,
                    onSelect = { controller.updateMaterial(selectedMat.copy(roughnessTexId = it)) },
                )
                TextureSlotSelector(
                    label = "AO Map",
                    selectedTexId = selectedMat.aoTexId,
                    textures = controller.textures,
                    onSelect = { controller.updateMaterial(selectedMat.copy(aoTexId = it)) },
                )
                TextureSlotSelector(
                    label = "Emissive Map",
                    selectedTexId = selectedMat.emissiveTexId,
                    textures = controller.textures,
                    onSelect = { controller.updateMaterial(selectedMat.copy(emissiveTexId = it)) },
                )

                val usedIds = setOf(
                    selectedMat.albedoTexId,
                    selectedMat.normalTexId,
                    selectedMat.roughnessTexId,
                    selectedMat.aoTexId,
                    selectedMat.emissiveTexId,
                ).filter { it > 0L }

                if (usedIds.isNotEmpty()) {
                    usedIds.forEach { tid ->
                        val tex = controller.textures.firstOrNull { it.id == tid } ?: return@forEach
                        Row(
                            modifier = Modifier.fillMaxWidth(),
                            horizontalArrangement = Arrangement.SpaceBetween,
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                text = "${tex.name} (${tex.width}x${tex.height}) ${if (tex.isMissing) "[MISSING]" else ""}",
                                color = if (tex.isMissing) NovaColors.WarningAmber else NovaColors.TextSecondary,
                                fontSize = 10.sp,
                                modifier = Modifier.weight(1f),
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            MatChip(
                                label = if (tex.isMissing) "Restore" else "Break Link",
                                active = tex.isMissing,
                                onClick = { controller.markTextureMissing(tex.id, !tex.isMissing) },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun TextureSlotSelector(
    label: String,
    selectedTexId: Long,
    textures: List<TextureUiSpec>,
    onSelect: (Long) -> Unit,
) {
    var open by remember { mutableStateOf(false) }
    val current = textures.firstOrNull { it.id == selectedTexId }
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, color = NovaColors.TextSecondary, fontSize = 11.sp)
        Box {
            MatChip(
                label = if (current == null) "None ▾" else "${current.name}${if (current.isMissing) " (!)" else ""} ▾",
                active = current != null,
                onClick = { open = true },
            )
            DropdownMenu(
                expanded = open,
                onDismissRequest = { open = false },
                containerColor = NovaColors.PanelBg,
            ) {
                DropdownMenuItem(
                    text = { Text("None", color = NovaColors.TextSecondary, fontSize = 12.sp) },
                    onClick = {
                        open = false
                        onSelect(0L)
                    },
                )
                textures.forEach { tex ->
                    DropdownMenuItem(
                        text = {
                            Text(
                                text = "${tex.name} (${tex.width}x${tex.height})${if (tex.isMissing) " [MISSING]" else ""}",
                                color = if (tex.isMissing) NovaColors.WarningAmber else NovaColors.TextPrimary,
                                fontSize = 12.sp,
                            )
                        },
                        onClick = {
                            open = false
                            onSelect(tex.id)
                        },
                    )
                }
            }
        }
    }
}

@Composable
private fun MatCard(
    title: String,
    content: @Composable () -> Unit,
) {
    Surface(
        color = NovaColors.CardBg,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Text(title, color = NovaColors.TextPrimary, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
            content()
        }
    }
}

@Composable
private fun MatSlider(
    label: String,
    value: Float,
    range: ClosedFloatingPointRange<Float>,
    onChange: (Float) -> Unit,
) {
    Column {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
        ) {
            Text(label, color = NovaColors.TextSecondary, fontSize = 11.sp)
            Text(
                String.format(Locale.US, "%.2f", value),
                color = NovaColors.TextPrimary,
                fontSize = 11.sp,
                fontFamily = FontFamily.Monospace,
            )
        }
        Slider(
            value = value.coerceIn(range.start, range.endInclusive),
            onValueChange = onChange,
            valueRange = range,
            colors = SliderDefaults.colors(
                thumbColor = NovaColors.AccentBlue,
                activeTrackColor = NovaColors.AccentBlue,
            ),
            modifier = Modifier.height(24.dp),
        )
    }
}

@Composable
private fun MatChip(
    label: String,
    active: Boolean,
    onClick: () -> Unit,
) {
    Surface(
        color = if (active) NovaColors.AccentBlue.copy(alpha = 0.24f) else NovaColors.PanelBg,
        shape = RoundedCornerShape(6.dp),
        border = BorderStroke(1.dp, if (active) NovaColors.AccentBlue else NovaColors.PanelBorder),
        modifier = Modifier.clip(RoundedCornerShape(6.dp)).clickable(onClick = onClick),
    ) {
        Text(
            text = label,
            color = if (active) Color.White else NovaColors.TextSecondary,
            fontSize = 11.sp,
            fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal,
            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
            maxLines = 1,
        )
    }
}
