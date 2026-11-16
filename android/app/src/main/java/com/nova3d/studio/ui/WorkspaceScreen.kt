package com.nova3d.studio.ui

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
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
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AccountTree
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.AspectRatio
import androidx.compose.material.icons.rounded.CallMade
import androidx.compose.material.icons.rounded.CallMerge
import androidx.compose.material.icons.rounded.CenterFocusStrong
import androidx.compose.material.icons.rounded.CenterFocusWeak
import androidx.compose.material.icons.rounded.ChevronLeft
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.ContentCopy
import androidx.compose.material.icons.rounded.ContentCut
import androidx.compose.material.icons.rounded.CropSquare
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.FiberManualRecord
import androidx.compose.material.icons.rounded.FilterCenterFocus
import androidx.compose.material.icons.rounded.Folder
import androidx.compose.material.icons.rounded.Fullscreen
import androidx.compose.material.icons.rounded.FullscreenExit
import androidx.compose.material.icons.rounded.Grid4x4
import androidx.compose.material.icons.rounded.GridOn
import androidx.compose.material.icons.rounded.Home
import androidx.compose.material.icons.rounded.KeyboardArrowDown
import androidx.compose.material.icons.rounded.KeyboardArrowUp
import androidx.compose.material.icons.rounded.Layers
import androidx.compose.material.icons.rounded.MoreHoriz
import androidx.compose.material.icons.rounded.MoreVert
import androidx.compose.material.icons.rounded.Movie
import androidx.compose.material.icons.rounded.NearMe
import androidx.compose.material.icons.rounded.OpenWith
import androidx.compose.material.icons.rounded.Palette
import androidx.compose.material.icons.rounded.PhotoCamera
import androidx.compose.material.icons.rounded.Redo
import androidx.compose.material.icons.rounded.RotateRight
import androidx.compose.material.icons.rounded.RoundedCorner
import androidx.compose.material.icons.rounded.Save
import androidx.compose.material.icons.rounded.Settings
import androidx.compose.material.icons.rounded.ShowChart
import androidx.compose.material.icons.rounded.Timeline
import androidx.compose.material.icons.rounded.Tonality
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material.icons.rounded.Undo
import androidx.compose.material.icons.rounded.Videocam
import androidx.compose.material.icons.rounded.ViewInAr
import androidx.compose.material.icons.rounded.WbSunny
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
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
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.render.DeviceRenderCapabilities
import com.nova3d.studio.render.OffscreenRenderEngine
import com.nova3d.studio.storage.ProjectStorage
import com.nova3d.studio.storage.ProjectSummary
import com.nova3d.studio.storage.RecoveryInfo
import com.nova3d.studio.ui.components.LandscapeTimelineStrip
import com.nova3d.studio.ui.components.NovaBrandMark
import com.nova3d.studio.ui.components.NovaDockToolButton
import com.nova3d.studio.ui.components.OrientationCubeWidget
import com.nova3d.studio.ui.components.ViewportActionIcon
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.EditSelectUiMode
import com.nova3d.studio.viewport.GizmoMode
import com.nova3d.studio.viewport.InteractionUiMode
import com.nova3d.studio.viewport.PrimitiveUiType
import com.nova3d.studio.viewport.ShadingUiMode
import com.nova3d.studio.viewport.ViewportController
import com.nova3d.studio.viewport.ViewportSurface
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.text.DateFormat
import java.util.Date
import java.util.Locale
import kotlin.math.roundToInt

enum class TopSection(val title: String, val workspaceLabel: String) {
    Home("Home", "Home"),
    Projects("Projects", "Projects"),
    Workspace("Workspace", "Modeling"),
    Materials("Materials", "Materials"),
    Animation("Animation", "Animation"),
    Render("Render", "Render"),
    Settings("Settings", "Settings"),
}

private enum class PortraitModalSheet {
    None,
    Outliner,
    Modifiers,
    World,
    MoreActions,
}

@Composable
fun NovaAppShell(
    controller: ViewportController,
    storage: ProjectStorage,
    isLifecycleActive: Boolean,
    modifier: Modifier = Modifier,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()

    var currentSection by remember { mutableStateOf(TopSection.Workspace) }
    var isFullscreen by remember { mutableStateOf(false) }
    var showSidePanel by remember { mutableStateOf(true) }

    var projects by remember { mutableStateOf<List<ProjectSummary>>(emptyList()) }
    var pendingRecovery by remember { mutableStateOf<RecoveryInfo?>(null) }
    var showSaveAsDialog by remember { mutableStateOf(false) }
    var pendingModelExportExt by remember { mutableStateOf("glb") }

    val importModelLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri: Uri? ->
        if (uri != null) {
            controller.import3dModelAsync(context, uri)
        }
    }

    val exportModelLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("*/*"),
    ) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        val fmt = pendingModelExportExt
        scope.launch {
            val (bytes, _) = controller.export3dModelBytes(fmt)
            if (bytes != null) {
                runCatching {
                    withContext(Dispatchers.IO) {
                        context.contentResolver.openOutputStream(uri)?.use { out ->
                            out.write(bytes)
                            out.flush()
                        } ?: error("Unable to open destination file")
                    }
                }.onSuccess {
                    controller.showStatus("Exported .${fmt.uppercase(Locale.US)} (${bytes.size} bytes)")
                }.onFailure { err ->
                    controller.showError("Export failed: ${err.message}")
                }
            }
        }
    }

    fun launchModelExport(ext: String) {
        pendingModelExportExt = ext
        val stem = controller.projectTitle
            .lowercase(Locale.US)
            .replace(Regex("[^a-z0-9]+"), "_")
            .trim('_')
            .ifBlank { "nova_scene" }
        exportModelLauncher.launch("$stem.$ext")
    }

    fun refreshProjects() {
        scope.launch {
            runCatching { storage.listProjects() }
                .onSuccess { projects = it }
                .onFailure { controller.showError("Failed to list projects: ${it.message}") }
        }
    }

    fun performSave(targetTitle: String, existingFileName: String?) {
        scope.launch {
            val payload = controller.serializeProjectBytes()
            if (payload == null) {
                controller.showError("Could not serialize current scene.")
                return@launch
            }
            val thumb = controller.captureThumbnailBitmap()
            runCatching {
                storage.saveProject(
                    title = targetTitle,
                    existingFileName = existingFileName,
                    payload = payload,
                    nodeCount = controller.sceneNodes.size,
                    thumbnail = thumb,
                )
            }.onSuccess { saved ->
                thumb?.recycle()
                controller.markSaved(saved.title, saved.fileName)
                storage.clearAutosave()
                storage.setSessionLock(true)
                refreshProjects()
            }.onFailure { err ->
                thumb?.recycle()
                controller.showError("Save failed: ${err.message ?: "unknown error"}")
            }
        }
    }

    fun quickSave() {
        val existing = controller.currentFileName
        if (existing != null) {
            performSave(controller.projectTitle, existing)
        } else {
            showSaveAsDialog = true
        }
    }

    val importLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            runCatching { storage.readUriBytes(context.contentResolver, uri) }
                .onSuccess { (rawName, bytes) ->
                    val title = rawName.removeSuffix(".nova3d").replace('_', ' ').ifBlank { "Imported Scene" }
                    if (controller.loadProjectBytes(title = title, fileName = null, bytes = bytes, markDirty = true)) {
                        controller.showStatus("Imported \"$title\"")
                        currentSection = TopSection.Workspace
                    }
                }
                .onFailure { err ->
                    controller.showError("Import failed: ${err.message ?: "cannot read file"}")
                }
        }
    }

    val exportLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.CreateDocument("application/octet-stream"),
    ) { uri: Uri? ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            val payload = controller.serializeProjectBytes()
            if (payload == null) {
                controller.showError("Could not serialize scene for export.")
                return@launch
            }
            runCatching { storage.writeUriBytes(context.contentResolver, uri, payload) }
                .onSuccess { controller.showStatus("Exported .nova3d file") }
                .onFailure { err -> controller.showError("Export failed: ${err.message ?: "write error"}") }
        }
    }

    LaunchedEffect(controller.isReady) {
        if (controller.isReady) {
            projects = runCatching { storage.listProjects() }.getOrDefault(emptyList())
            val rec = runCatching { storage.getPendingRecovery() }.getOrNull()
            if (rec != null) {
                pendingRecovery = rec
            }
            storage.setSessionLock(true)
        }
    }

    LaunchedEffect(controller.isReady, controller.hasUnsavedChanges, controller.sceneNodes, controller.selectedNodeDetails) {
        if (controller.isReady && controller.hasUnsavedChanges) {
            delay(2500)
            val payload = controller.serializeProjectBytes()
            if (payload != null) {
                runCatching {
                    storage.writeAutosave(
                        title = controller.projectTitle,
                        linkedFileName = controller.currentFileName,
                        dirty = true,
                        payload = payload,
                    )
                }
            }
        }
    }

    LaunchedEffect(isLifecycleActive) {
        if (!isLifecycleActive && controller.isReady) {
            if (controller.hasUnsavedChanges) {
                val payload = controller.serializeProjectBytes()
                if (payload != null) {
                    runCatching {
                        storage.writeAutosave(
                            title = controller.projectTitle,
                            linkedFileName = controller.currentFileName,
                            dirty = true,
                            payload = payload,
                        )
                    }
                }
            } else {
                runCatching { storage.setSessionLock(false) }
            }
        } else if (isLifecycleActive && controller.isReady) {
            runCatching { storage.setSessionLock(true) }
        }
    }

    LaunchedEffect(controller.statusBanner) {
        if (controller.statusBanner != null) {
            delay(2500)
            controller.clearStatusBanner()
        }
    }

    BoxWithConstraints(
        modifier = modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg),
    ) {
        val isWide = maxWidth >= 680.dp || (maxWidth > maxHeight && maxWidth >= 520.dp)

        Column(modifier = Modifier.fillMaxSize()) {
            if (!isFullscreen) {
                NovaTopHeader(
                    controller = controller,
                    currentSection = currentSection,
                    isWide = isWide,
                    showSidePanel = showSidePanel,
                    onSelectSection = { currentSection = it },
                    onSaveQuick = { quickSave() },
                    onSaveAs = { showSaveAsDialog = true },
                    onImport3dModel = { importModelLauncher.launch(arrayOf("*/*")) },
                    onExportGlb = { launchModelExport("glb") },
                    onExportObj = { launchModelExport("obj") },
                    onToggleSidePanel = { showSidePanel = !showSidePanel },
                    onToggleFullscreen = { isFullscreen = !isFullscreen },
                )
            }

            Box(modifier = Modifier.weight(1f).fillMaxWidth()) {
                when (currentSection) {
                    TopSection.Home -> HomeScreen(
                        controller = controller,
                        projects = projects,
                        onNewEmpty = {
                            controller.newEmptyProject("Untitled Scene")
                            scope.launch { storage.clearAutosave(); storage.setSessionLock(true) }
                            currentSection = TopSection.Workspace
                        },
                        onNewStarter = {
                            controller.newStarterProject("Studio Starter")
                            scope.launch { storage.clearAutosave(); storage.setSessionLock(true) }
                            currentSection = TopSection.Workspace
                        },
                        onOpenWorkspace = { currentSection = TopSection.Workspace },
                        onOpenProject = { summary ->
                            scope.launch {
                                runCatching { storage.loadProjectBytes(summary.fileName) }
                                    .onSuccess { bytes ->
                                        if (controller.loadProjectBytes(summary.title, summary.fileName, bytes, markDirty = false)) {
                                            storage.clearAutosave()
                                            storage.setSessionLock(true)
                                            controller.showStatus("Opened \"${summary.title}\"")
                                            currentSection = TopSection.Workspace
                                        }
                                    }
                                    .onFailure { err ->
                                        controller.showError("Open failed: ${err.message}")
                                    }
                            }
                        },
                        onImportNova3d = {
                            importLauncher.launch(arrayOf("*/*"))
                        },
                    )

                    TopSection.Projects -> ProjectsScreen(
                        controller = controller,
                        projects = projects,
                        onSaveCurrent = { quickSave() },
                        onSaveAs = { showSaveAsDialog = true },
                        onOpenProject = { summary ->
                            scope.launch {
                                runCatching { storage.loadProjectBytes(summary.fileName) }
                                    .onSuccess { bytes ->
                                        if (controller.loadProjectBytes(summary.title, summary.fileName, bytes, markDirty = false)) {
                                            storage.clearAutosave()
                                            storage.setSessionLock(true)
                                            controller.showStatus("Opened \"${summary.title}\"")
                                            currentSection = TopSection.Workspace
                                        }
                                    }
                                    .onFailure { err ->
                                        controller.showError("Open failed: ${err.message}")
                                    }
                            }
                        },
                        onDeleteProject = { summary ->
                            scope.launch {
                                storage.deleteProject(summary.fileName)
                                refreshProjects()
                                controller.showStatus("Deleted \"${summary.title}\"")
                            }
                        },
                        onImportNova3d = { importLauncher.launch(arrayOf("*/*")) },
                        onExportNova3d = {
                            val suggested = controller.currentFileName
                                ?: (controller.projectTitle.lowercase(Locale.US).replace(Regex("[^a-z0-9]+"), "_").trim('_').ifBlank { "scene" } + ".nova3d")
                            exportLauncher.launch(suggested)
                        },
                        onImport3dModel = { importModelLauncher.launch(arrayOf("*/*")) },
                        onExportGlb = { launchModelExport("glb") },
                        onExportObj = { launchModelExport("obj") },
                    )

                    TopSection.Workspace -> WorkspaceContent(
                        controller = controller,
                        isLifecycleActive = isLifecycleActive,
                        isFullscreen = isFullscreen,
                        isWide = isWide,
                        showSidePanel = showSidePanel,
                        onToggleFullscreen = { isFullscreen = !isFullscreen },
                        onToggleSidePanel = { showSidePanel = !showSidePanel },
                        onImport3dModel = { importModelLauncher.launch(arrayOf("*/*")) },
                        onNavigateToSection = { currentSection = it },
                    )

                    TopSection.Materials -> MaterialsScreen(
                        controller = controller,
                        isLifecycleActive = isLifecycleActive,
                    )

                    TopSection.Animation -> AnimationScreen(
                        controller = controller,
                        isLifecycleActive = isLifecycleActive,
                    )

                    TopSection.Render -> RenderScreen(
                        controller = controller,
                        onExportGlb = { launchModelExport("glb") },
                        onExportGltf = { launchModelExport("gltf") },
                        onExportObj = { launchModelExport("obj") },
                    )

                    TopSection.Settings -> SettingsScreen(controller = controller)
                }

                // Exit Fullscreen subtle floating button when in fullscreen mode
                if (isFullscreen) {
                    Surface(
                        color = NovaColors.PanelBg.copy(alpha = 0.85f),
                        shape = CircleShape,
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(10.dp)
                            .size(34.dp)
                            .clip(CircleShape)
                            .clickable { isFullscreen = false },
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Icon(
                                imageVector = Icons.Rounded.FullscreenExit,
                                contentDescription = "Exit Fullscreen",
                                tint = NovaColors.TextPrimary,
                                modifier = Modifier.size(18.dp),
                            )
                        }
                    }
                }
            }
        }
    }

    if (controller.lastIoWarnings.isNotEmpty()) {
        AlertDialog(
            onDismissRequest = { controller.clearIoWarnings() },
            containerColor = NovaColors.PanelBg,
            title = {
                Text(
                    text = "3D Format Conversion Notes",
                    color = NovaColors.WarningAmber,
                    fontSize = 16.sp,
                    fontWeight = FontWeight.SemiBold,
                )
            },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    controller.lastIoWarnings.forEach { w ->
                        Text(text = "• $w", color = NovaColors.TextPrimary, fontSize = 12.sp)
                    }
                }
            },
            confirmButton = {
                TextButton(onClick = { controller.clearIoWarnings() }) {
                    Text("Got It", color = NovaColors.AccentViolet)
                }
            },
        )
    }

    val recovery = pendingRecovery
    if (recovery != null) {
        val updatedLabel = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
            .format(Date(recovery.updatedAtMillis))
        AlertDialog(
            onDismissRequest = {},
            containerColor = NovaColors.PanelBg,
            title = {
                Text("Recover Unsaved Session?", color = NovaColors.TextPrimary, fontSize = 16.sp)
            },
            text = {
                Text(
                    text = "An autosaved session for \"${recovery.title}\" (saved $updatedLabel, ${recovery.sizeBytes / 1024} KB) was found. Restore it now?",
                    color = NovaColors.TextSecondary,
                    fontSize = 13.sp,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingRecovery = null
                        scope.launch {
                            runCatching { storage.loadRecoveryBytes() }
                                .onSuccess { bytes ->
                                    if (controller.loadProjectBytes(
                                            title = recovery.title,
                                            fileName = recovery.linkedFileName,
                                            bytes = bytes,
                                            markDirty = true,
                                        )
                                    ) {
                                        controller.showStatus("Recovered \"${recovery.title}\"")
                                    }
                                }
                                .onFailure { err ->
                                    controller.showError("Recovery failed: ${err.message}")
                                }
                        }
                    },
                ) {
                    Text("Recover", color = NovaColors.AccentViolet)
                }
            },
            dismissButton = {
                TextButton(
                    onClick = {
                        pendingRecovery = null
                        scope.launch { storage.clearAutosave(); storage.setSessionLock(true) }
                    },
                ) {
                    Text("Discard", color = NovaColors.TextSecondary)
                }
            },
        )
    }

    if (showSaveAsDialog) {
        SaveAsProjectDialog(
            initialTitle = controller.projectTitle,
            onDismiss = { showSaveAsDialog = false },
            onConfirm = { newTitle ->
                showSaveAsDialog = false
                performSave(newTitle, null)
            },
        )
    }
}

@Composable
private fun SaveAsProjectDialog(
    initialTitle: String,
    onDismiss: () -> Unit,
    onConfirm: (String) -> Unit,
) {
    var title by remember(initialTitle) { mutableStateOf(initialTitle) }
    AlertDialog(
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
        title = { Text("Save Project (.nova3d)", color = NovaColors.TextPrimary, fontSize = 16.sp) },
        text = {
            OutlinedTextField(
                value = title,
                onValueChange = { title = it },
                singleLine = true,
                label = { Text("Project Name") },
            )
        },
        confirmButton = {
            TextButton(
                onClick = { if (title.isNotBlank()) onConfirm(title.trim()) },
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
private fun NovaTopHeader(
    controller: ViewportController,
    currentSection: TopSection,
    isWide: Boolean,
    showSidePanel: Boolean,
    onSelectSection: (TopSection) -> Unit,
    onSaveQuick: () -> Unit,
    onSaveAs: () -> Unit,
    onImport3dModel: () -> Unit,
    onExportGlb: () -> Unit,
    onExportObj: () -> Unit,
    onToggleSidePanel: () -> Unit,
    onToggleFullscreen: () -> Unit,
) {
    var overflowMenuOpen by remember { mutableStateOf(false) }

    Surface(
        color = NovaColors.PanelBg,
        modifier = Modifier
            .fillMaxWidth()
            .then(if (!isWide) Modifier.statusBarsPadding() else Modifier),
    ) {
        Column {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(48.dp)
                    .padding(horizontal = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                // Brand Mark + NOVA 3D (clickable to Home)
                Row(
                    modifier = Modifier
                        .clip(RoundedCornerShape(8.dp))
                        .clickable { onSelectSection(TopSection.Home) }
                        .padding(horizontal = 4.dp, vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(7.dp),
                ) {
                    NovaBrandMark(size = 22.dp)
                    Text(
                        text = buildAnnotatedString {
                            withStyle(SpanStyle(color = NovaColors.TextPrimary, fontWeight = FontWeight.Bold)) {
                                append("NOVA ")
                            }
                            withStyle(SpanStyle(color = NovaColors.AccentVioletGlow, fontWeight = FontWeight.Bold)) {
                                append("3D")
                            }
                        },
                        fontSize = 14.sp,
                        letterSpacing = 0.4.sp,
                        maxLines = 1,
                    )
                }

                // Vertical hairline divider
                Box(
                    modifier = Modifier
                        .width(1.dp)
                        .height(18.dp)
                        .background(NovaColors.PanelBorder),
                )

                if (isWide) {
                    // Landscape: Project title + saved dot + Workspace Switcher Tabs + right actions
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(6.dp),
                        modifier = Modifier.width(110.dp),
                    ) {
                        Text(
                            text = controller.projectTitle,
                            color = NovaColors.TextSecondary,
                            fontSize = 12.sp,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            modifier = Modifier.weight(1f, fill = false),
                        )
                        Box(
                            modifier = Modifier
                                .size(7.dp)
                                .clip(CircleShape)
                                .background(
                                    if (controller.hasUnsavedChanges) NovaColors.AccentVioletGlow
                                    else NovaColors.SuccessGreen,
                                ),
                        )
                    }

                    // Workspace Switcher Pills (Reference #3)
                    Row(
                        modifier = Modifier
                            .weight(1f)
                            .horizontalScroll(rememberScrollState()),
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        listOf(
                            TopSection.Workspace,
                            TopSection.Materials,
                            TopSection.Animation,
                            TopSection.Render,
                            TopSection.Projects,
                            TopSection.Settings,
                        ).forEach { sec ->
                            val active = sec == currentSection
                            Surface(
                                color = if (active) NovaColors.AccentViolet else Color.Transparent,
                                shape = RoundedCornerShape(8.dp),
                                modifier = Modifier
                                    .clip(RoundedCornerShape(8.dp))
                                    .clickable { onSelectSection(sec) },
                            ) {
                                Text(
                                    text = sec.workspaceLabel,
                                    color = if (active) Color.White else NovaColors.TextSecondary,
                                    fontSize = 12.sp,
                                    fontWeight = if (active) FontWeight.SemiBold else FontWeight.Medium,
                                    maxLines = 1,
                                    modifier = Modifier.padding(horizontal = 11.dp, vertical = 5.dp),
                                )
                            }
                        }
                    }

                    // Action Buttons
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        HeaderIconButton(
                            icon = Icons.Rounded.Undo,
                            contentDescription = "Undo",
                            enabled = controller.canUndo,
                            onClick = { controller.undo() },
                        )
                        HeaderIconButton(
                            icon = Icons.Rounded.Redo,
                            contentDescription = "Redo",
                            enabled = controller.canRedo,
                            onClick = { controller.redo() },
                        )
                        HeaderIconButton(
                            icon = Icons.Rounded.Save,
                            contentDescription = "Save",
                            tint = if (controller.hasUnsavedChanges) NovaColors.AccentVioletGlow else NovaColors.TextSecondary,
                            onClick = onSaveQuick,
                        )
                        if (currentSection == TopSection.Workspace) {
                            HeaderIconButton(
                                icon = if (showSidePanel) Icons.Rounded.ChevronRight else Icons.Rounded.ChevronLeft,
                                contentDescription = "Toggle Side Panel",
                                onClick = onToggleSidePanel,
                            )
                        }
                        HeaderIconButton(
                            icon = Icons.Rounded.Fullscreen,
                            contentDescription = "Fullscreen",
                            onClick = onToggleFullscreen,
                        )
                    }
                } else {
                    // Portrait: Slim Header matching References #1 & #2
                    Column(
                        modifier = Modifier
                            .weight(1f)
                            .clickable { onSelectSection(TopSection.Projects) },
                    ) {
                        Text(
                            text = controller.projectTitle,
                            color = NovaColors.TextPrimary,
                            fontSize = 12.sp,
                            fontWeight = FontWeight.Medium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Box(
                                modifier = Modifier
                                    .size(6.dp)
                                    .clip(CircleShape)
                                    .background(
                                        if (controller.hasUnsavedChanges) NovaColors.AccentVioletGlow
                                        else NovaColors.SuccessGreen,
                                    ),
                            )
                            Text(
                                text = if (controller.hasUnsavedChanges) "unsaved" else "Saved",
                                color = if (controller.hasUnsavedChanges) NovaColors.AccentVioletGlow else NovaColors.TextMuted,
                                fontSize = 10.sp,
                            )
                        }
                    }

                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        HeaderIconButton(
                            icon = Icons.Rounded.Undo,
                            contentDescription = "Undo",
                            enabled = controller.canUndo,
                            onClick = { controller.undo() },
                        )
                        HeaderIconButton(
                            icon = Icons.Rounded.Redo,
                            contentDescription = "Redo",
                            enabled = controller.canRedo,
                            onClick = { controller.redo() },
                        )
                        HeaderIconButton(
                            icon = Icons.Rounded.Save,
                            contentDescription = "Save",
                            tint = if (controller.hasUnsavedChanges) NovaColors.AccentVioletGlow else NovaColors.TextSecondary,
                            onClick = onSaveQuick,
                        )
                    }
                }

                // Overflow Menu (always accessible in both orientations)
                Box {
                    HeaderIconButton(
                        icon = Icons.Rounded.MoreVert,
                        contentDescription = "Workspace Menu",
                        onClick = { overflowMenuOpen = true },
                    )
                    DropdownMenu(
                        expanded = overflowMenuOpen,
                        onDismissRequest = { overflowMenuOpen = false },
                        containerColor = NovaColors.PanelBg,
                    ) {
                        listOf(
                            Triple(TopSection.Home, "Home Dashboard", Icons.Rounded.Home),
                            Triple(TopSection.Workspace, "Modeling Viewport", Icons.Rounded.ViewInAr),
                            Triple(TopSection.Materials, "Materials Studio", Icons.Rounded.Palette),
                            Triple(TopSection.Animation, "Animation & Timeline", Icons.Rounded.Movie),
                            Triple(TopSection.Render, "Final Render & MP4", Icons.Rounded.PhotoCamera),
                            Triple(TopSection.Projects, "Projects Library", Icons.Rounded.Folder),
                            Triple(TopSection.Settings, "Engine & Settings", Icons.Rounded.Settings),
                        ).forEach { (sec, label, icon) ->
                            val selected = currentSection == sec
                            DropdownMenuItem(
                                leadingIcon = {
                                    Icon(
                                        imageVector = icon,
                                        contentDescription = null,
                                        tint = if (selected) NovaColors.AccentVioletGlow else NovaColors.TextSecondary,
                                        modifier = Modifier.size(18.dp),
                                    )
                                },
                                text = {
                                    Text(
                                        text = label,
                                        color = if (selected) NovaColors.AccentVioletGlow else NovaColors.TextPrimary,
                                        fontSize = 13.sp,
                                        fontWeight = if (selected) FontWeight.SemiBold else FontWeight.Normal,
                                    )
                                },
                                onClick = {
                                    overflowMenuOpen = false
                                    onSelectSection(sec)
                                },
                            )
                        }
                        HorizontalDivider(color = NovaColors.PanelBorder)
                        DropdownMenuItem(
                            text = { Text("Save As (.nova3d)", color = NovaColors.TextPrimary, fontSize = 13.sp) },
                            onClick = { overflowMenuOpen = false; onSaveAs() },
                        )
                        DropdownMenuItem(
                            text = { Text("Import 3D (GLB/glTF/OBJ)", color = NovaColors.AccentCyan, fontSize = 13.sp) },
                            onClick = { overflowMenuOpen = false; onImport3dModel() },
                        )
                        DropdownMenuItem(
                            text = { Text("Export .GLB", color = NovaColors.TextPrimary, fontSize = 13.sp) },
                            onClick = { overflowMenuOpen = false; onExportGlb() },
                        )
                        DropdownMenuItem(
                            text = { Text("Export .OBJ", color = NovaColors.TextPrimary, fontSize = 13.sp) },
                            onClick = { overflowMenuOpen = false; onExportObj() },
                        )
                        DropdownMenuItem(
                            text = { Text("Fullscreen Viewport", color = NovaColors.TextSecondary, fontSize = 13.sp) },
                            onClick = { overflowMenuOpen = false; onToggleFullscreen() },
                        )
                    }
                }
            }
            HorizontalDivider(color = NovaColors.PanelBorder, thickness = 1.dp)
        }
    }
}

@Composable
private fun HeaderIconButton(
    icon: ImageVector,
    contentDescription: String,
    enabled: Boolean = true,
    tint: Color = NovaColors.TextSecondary,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .size(32.dp)
            .clip(RoundedCornerShape(8.dp))
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = contentDescription,
            tint = if (enabled) tint else NovaColors.TextMuted.copy(alpha = 0.35f),
            modifier = Modifier.size(19.dp),
        )
    }
}

@Composable
private fun WorkspaceContent(
    controller: ViewportController,
    isLifecycleActive: Boolean,
    isFullscreen: Boolean,
    isWide: Boolean,
    showSidePanel: Boolean,
    onToggleFullscreen: () -> Unit,
    onToggleSidePanel: () -> Unit,
    onImport3dModel: () -> Unit,
    onNavigateToSection: (TopSection) -> Unit,
) {
    if (isWide) {
        LandscapeWorkspaceContent(
            controller = controller,
            isLifecycleActive = isLifecycleActive,
            isFullscreen = isFullscreen,
            showSidePanel = showSidePanel,
            onToggleSidePanel = onToggleSidePanel,
            onImport3dModel = onImport3dModel,
        )
    } else {
        PortraitWorkspaceContent(
            controller = controller,
            isLifecycleActive = isLifecycleActive,
            isFullscreen = isFullscreen,
            onImport3dModel = onImport3dModel,
            onNavigateToSection = onNavigateToSection,
        )
    }
}

/**
 * PORTRAIT WORKSPACE (References #1 & #2):
 * Full-bleed viewport (75-85%+ primary area), floating top mode pill, floating right action pill,
 * orientation cube, floating bottom contextual card / edit sheet, and floating bottom tool dock.
 */
@Composable
private fun PortraitWorkspaceContent(
    controller: ViewportController,
    isLifecycleActive: Boolean,
    isFullscreen: Boolean,
    onImport3dModel: () -> Unit,
    onNavigateToSection: (TopSection) -> Unit,
) {
    var addMenuExpanded by remember { mutableStateOf(false) }
    var modeMenuExpanded by remember { mutableStateOf(false) }
    var shadingMenuExpanded by remember { mutableStateOf(false) }
    var overlaysMenuExpanded by remember { mutableStateOf(false) }

    var cardExpanded by remember { mutableStateOf(true) }
    var showCompactTimeline by remember { mutableStateOf(false) }
    var activeModalSheet by remember { mutableStateOf(PortraitModalSheet.None) }

    val inEditMode = controller.editState.interactionMode == InteractionUiMode.Edit
    val es = controller.editState
    val selectedNode = controller.sceneNodes.firstOrNull { it.primarySelected }
    val activeCam = controller.sceneNodes.firstOrNull { it.id == controller.timeline.activeCameraNodeId }

    Box(modifier = Modifier.fillMaxSize().background(NovaColors.CanvasBg)) {
        // Primary Fullscreen 3D Viewport
        ViewportSurface(
            controller = controller,
            isActive = isLifecycleActive,
            modifier = Modifier.fillMaxSize(),
        )

        // Top-Left Telemetry HUD (Crisp minimal overlay)
        Column(
            modifier = Modifier
                .align(Alignment.TopStart)
                .padding(start = 10.dp, top = 10.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            if (inEditMode) {
                Text(
                    text = "Edit Mode (${es.selectMode.label})",
                    color = NovaColors.AccentVioletGlow,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = "Polys: ${es.totalFaces} | Verts: ${es.totalVerts}",
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                )
            } else {
                Text(
                    text = "${controller.activeBackendName} • ${controller.fps} FPS",
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                )
                Text(
                    text = "Sel: ${selectedNode?.name ?: "None"}",
                    color = NovaColors.TextMuted,
                    fontSize = 10.sp,
                )
            }
        }

        // Top-Center Floating Mode Pill (Reference #1: "Object Mode v" / Reference #2: "Edit Mode | Vertex Edge Face")
        if (!isFullscreen) {
            Box(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 8.dp),
            ) {
                if (!inEditMode) {
                    Surface(
                        color = NovaColors.PanelBg.copy(alpha = 0.88f),
                        shape = RoundedCornerShape(12.dp),
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        modifier = Modifier
                            .clip(RoundedCornerShape(12.dp))
                            .clickable { modeMenuExpanded = true },
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 12.dp, vertical = 6.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(5.dp),
                        ) {
                            Text(
                                text = "Object Mode",
                                color = NovaColors.TextPrimary,
                                fontSize = 12.sp,
                                fontWeight = FontWeight.Medium,
                            )
                            Icon(
                                imageVector = Icons.Rounded.KeyboardArrowDown,
                                contentDescription = null,
                                tint = NovaColors.TextSecondary,
                                modifier = Modifier.size(16.dp),
                            )
                        }
                    }

                    DropdownMenu(
                        expanded = modeMenuExpanded,
                        onDismissRequest = { modeMenuExpanded = false },
                        containerColor = NovaColors.PanelBg,
                    ) {
                        DropdownMenuItem(
                            text = { Text("Object Mode", color = NovaColors.AccentVioletGlow, fontSize = 13.sp) },
                            onClick = {
                                modeMenuExpanded = false
                                controller.setInteractionMode(InteractionUiMode.Object)
                            },
                        )
                        DropdownMenuItem(
                            text = { Text("Edit Mode (Mesh)", color = NovaColors.TextPrimary, fontSize = 13.sp) },
                            onClick = {
                                modeMenuExpanded = false
                                controller.setInteractionMode(InteractionUiMode.Edit)
                            },
                        )
                        HorizontalDivider(color = NovaColors.PanelBorder)
                        DropdownMenuItem(
                            text = {
                                Text(
                                    "Gizmo Space: ${controller.gizmoSpace.label}",
                                    color = NovaColors.TextSecondary,
                                    fontSize = 12.sp,
                                )
                            },
                            onClick = {
                                modeMenuExpanded = false
                                controller.toggleGizmoSpace()
                            },
                        )
                        DropdownMenuItem(
                            text = {
                                Text(
                                    "Multi-Select: ${if (controller.multiSelectEnabled) "ON" else "OFF"}",
                                    color = NovaColors.TextSecondary,
                                    fontSize = 12.sp,
                                )
                            },
                            onClick = {
                                modeMenuExpanded = false
                                controller.toggleMultiSelect()
                            },
                        )
                    }
                } else {
                    // Edit Mode floating capsule with Vertex / Edge / Face selector (Reference #2!)
                    Surface(
                        color = NovaColors.PanelBg.copy(alpha = 0.92f),
                        shape = RoundedCornerShape(14.dp),
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 6.dp, vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            Surface(
                                color = NovaColors.AccentVioletSoft,
                                shape = RoundedCornerShape(10.dp),
                                border = BorderStroke(1.dp, NovaColors.AccentViolet),
                                modifier = Modifier
                                    .clip(RoundedCornerShape(10.dp))
                                    .clickable { controller.setInteractionMode(InteractionUiMode.Object) },
                            ) {
                                Text(
                                    text = "Edit (${es.selectMode.label})",
                                    color = Color.White,
                                    fontSize = 11.sp,
                                    fontWeight = FontWeight.SemiBold,
                                    modifier = Modifier.padding(horizontal = 9.dp, vertical = 5.dp),
                                )
                            }

                            EditSelectUiMode.entries.forEach { sm ->
                                val active = es.selectMode == sm
                                val icon = when (sm) {
                                    EditSelectUiMode.Vertex -> Icons.Rounded.FiberManualRecord
                                    EditSelectUiMode.Edge -> Icons.Rounded.ShowChart
                                    EditSelectUiMode.Face -> Icons.Rounded.CropSquare
                                }
                                Row(
                                    modifier = Modifier
                                        .clip(RoundedCornerShape(8.dp))
                                        .background(if (active) NovaColors.AccentViolet else Color.Transparent)
                                        .clickable { controller.setEditSelectMode(sm) }
                                        .padding(horizontal = 7.dp, vertical = 4.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                    horizontalArrangement = Arrangement.spacedBy(3.dp),
                                ) {
                                    Icon(
                                        imageVector = icon,
                                        contentDescription = sm.label,
                                        tint = if (active) Color.White else NovaColors.TextSecondary,
                                        modifier = Modifier.size(13.dp),
                                    )
                                    Text(
                                        text = sm.label,
                                        color = if (active) Color.White else NovaColors.TextSecondary,
                                        fontSize = 10.sp,
                                        fontWeight = if (active) FontWeight.SemiBold else FontWeight.Normal,
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }

        // Top-Right: View label + 3D Orientation Cube + Vertical Action Pill
        Column(
            modifier = Modifier
                .align(Alignment.TopEnd)
                .padding(top = 8.dp, end = 8.dp),
            horizontalAlignment = Alignment.End,
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (!isFullscreen) {
                Text(
                    text = "${controller.projectionMode.label} • ${activeCam?.name ?: "Orbit"}",
                    color = NovaColors.TextSecondary,
                    fontSize = 10.sp,
                    modifier = Modifier
                        .clip(RoundedCornerShape(6.dp))
                        .clickable { controller.toggleProjection() }
                        .padding(horizontal = 4.dp, vertical = 2.dp),
                )
            }

            OrientationCubeWidget(
                yawDeg = controller.cameraYawDeg,
                pitchDeg = controller.cameraPitchDeg,
                onSelectViewPreset = { controller.setStandardView(it) },
            )

            // Floating Right Vertical Viewport Pill (Reference #1 & #2)
            if (!isFullscreen) {
                Surface(
                    color = NovaColors.PanelBg.copy(alpha = 0.88f),
                    shape = RoundedCornerShape(12.dp),
                    border = BorderStroke(1.dp, NovaColors.PanelBorder),
                ) {
                    Column(
                        modifier = Modifier.padding(3.dp),
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        // Shading Mode Menu
                        Box {
                            ViewportActionIcon(
                                icon = Icons.Rounded.Tonality,
                                contentDescription = "Shading Mode",
                                active = controller.shadingMode != ShadingUiMode.Solid,
                                onClick = { shadingMenuExpanded = true },
                            )
                            DropdownMenu(
                                expanded = shadingMenuExpanded,
                                onDismissRequest = { shadingMenuExpanded = false },
                                containerColor = NovaColors.PanelBg,
                            ) {
                                ShadingUiMode.entries.forEach { mode ->
                                    DropdownMenuItem(
                                        text = {
                                            Text(
                                                text = when (mode) {
                                                    ShadingUiMode.Solid -> "Solid Clay"
                                                    ShadingUiMode.Wireframe -> "Wireframe"
                                                    ShadingUiMode.MaterialPreview -> "Material Preview"
                                                    ShadingUiMode.RenderPreview -> "Render Preview (HDR)"
                                                },
                                                color = if (controller.shadingMode == mode) NovaColors.AccentVioletGlow else NovaColors.TextPrimary,
                                                fontSize = 12.sp,
                                            )
                                        },
                                        onClick = {
                                            shadingMenuExpanded = false
                                            controller.setShadingMode(mode)
                                        },
                                    )
                                }
                            }
                        }

                        // Overlays (Grid / Axes / Face Orientation)
                        Box {
                            ViewportActionIcon(
                                icon = Icons.Rounded.GridOn,
                                contentDescription = "Overlays",
                                active = controller.showGrid || controller.showAxes,
                                onClick = { overlaysMenuExpanded = true },
                            )
                            DropdownMenu(
                                expanded = overlaysMenuExpanded,
                                onDismissRequest = { overlaysMenuExpanded = false },
                                containerColor = NovaColors.PanelBg,
                            ) {
                                DropdownMenuItem(
                                    text = { Text("Grid: ${if (controller.showGrid) "ON" else "OFF"}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                    onClick = { controller.toggleGrid() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Axes: ${if (controller.showAxes) "ON" else "OFF"}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                    onClick = { controller.toggleAxes() },
                                )
                                DropdownMenuItem(
                                    text = { Text("Face Orient: ${if (es.showFaceOrientation) "ON" else "OFF"}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                    onClick = { controller.toggleFaceOrientation() },
                                )
                            }
                        }

                        ViewportActionIcon(
                            icon = Icons.Rounded.CenterFocusWeak,
                            contentDescription = "Frame All",
                            onClick = { controller.frameAll() },
                        )
                        ViewportActionIcon(
                            icon = Icons.Rounded.FilterCenterFocus,
                            contentDescription = "Focus Selected",
                            onClick = { controller.focusSelected() },
                        )
                        ViewportActionIcon(
                            icon = Icons.Rounded.Videocam,
                            contentDescription = "Toggle Camera View",
                            active = controller.timeline.activeCameraNodeId != 0L,
                            onClick = { controller.toggleCameraView() },
                        )
                        ViewportActionIcon(
                            icon = Icons.Rounded.ViewInAr,
                            contentDescription = "Persp / Ortho",
                            active = controller.projectionMode.code == 1,
                            onClick = { controller.toggleProjection() },
                        )
                    }
                }
            }
        }

        // Transient Status Banner
        val banner = controller.statusBanner
        if (banner != null) {
            Surface(
                color = NovaColors.PanelBg.copy(alpha = 0.94f),
                shape = RoundedCornerShape(10.dp),
                border = BorderStroke(1.dp, NovaColors.AccentViolet),
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(top = 56.dp),
            ) {
                Text(
                    text = banner,
                    color = NovaColors.TextPrimary,
                    fontSize = 12.sp,
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp),
                )
            }
        }

        // Bottom Contextual Stack + Floating Tool Dock
        if (!isFullscreen) {
            Column(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .padding(horizontal = 10.dp, vertical = 10.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                // Optional Modal Sheet (Outliner / Modifiers / World / More Actions)
                if (activeModalSheet != PortraitModalSheet.None) {
                    Surface(
                        color = NovaColors.PanelBg.copy(alpha = 0.97f),
                        shape = RoundedCornerShape(16.dp),
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        shadowElevation = 14.dp,
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(max = 310.dp),
                    ) {
                        Column(modifier = Modifier.fillMaxWidth().padding(12.dp)) {
                            Row(
                                modifier = Modifier.fillMaxWidth(),
                                horizontalArrangement = Arrangement.SpaceBetween,
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(
                                    text = when (activeModalSheet) {
                                        PortraitModalSheet.Outliner -> "Scene Hierarchy (${controller.sceneNodes.size})"
                                        PortraitModalSheet.Modifiers -> "Modifier Stack"
                                        PortraitModalSheet.World -> "World & Lighting"
                                        PortraitModalSheet.MoreActions -> "Quick Tools & Panels"
                                        PortraitModalSheet.None -> ""
                                    },
                                    color = NovaColors.TextPrimary,
                                    fontSize = 13.sp,
                                    fontWeight = FontWeight.SemiBold,
                                )
                                Icon(
                                    imageVector = Icons.Rounded.Close,
                                    contentDescription = "Close",
                                    tint = NovaColors.TextSecondary,
                                    modifier = Modifier
                                        .size(20.dp)
                                        .clip(CircleShape)
                                        .clickable { activeModalSheet = PortraitModalSheet.None },
                                )
                            }
                            Spacer(Modifier.height(8.dp))
                            when (activeModalSheet) {
                                PortraitModalSheet.Outliner -> OutlinerTabContent(
                                    controller = controller,
                                    onJumpToProperties = {
                                        activeModalSheet = PortraitModalSheet.None
                                        cardExpanded = true
                                    },
                                    modifier = Modifier.fillMaxWidth().height(230.dp),
                                )
                                PortraitModalSheet.Modifiers -> Column(
                                    modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState()),
                                ) {
                                    ModifierStackCard(controller = controller, embedInline = true)
                                }
                                PortraitModalSheet.World -> Column(
                                    modifier = Modifier.fillMaxWidth().verticalScroll(rememberScrollState()),
                                ) {
                                    WorldLightingTabContent(controller = controller, embedInline = true)
                                }
                                PortraitModalSheet.MoreActions -> PortraitMoreToolsGrid(
                                    controller = controller,
                                    showCompactTimeline = showCompactTimeline,
                                    onOpenOutliner = { activeModalSheet = PortraitModalSheet.Outliner },
                                    onOpenWorld = { activeModalSheet = PortraitModalSheet.World },
                                    onOpenModifiers = { activeModalSheet = PortraitModalSheet.Modifiers },
                                    onToggleTimeline = {
                                        showCompactTimeline = !showCompactTimeline
                                        activeModalSheet = PortraitModalSheet.None
                                    },
                                    onNavigate = { sec ->
                                        activeModalSheet = PortraitModalSheet.None
                                        onNavigateToSection(sec)
                                    },
                                )
                                PortraitModalSheet.None -> {}
                            }
                        }
                    }
                } else {
                    // Contextual Card: Edit Mode Bottom Sheet OR Selected Object Floating Card
                    if (inEditMode) {
                        PortraitEditModeBottomSheet(
                            controller = controller,
                            expanded = cardExpanded,
                            onToggleExpanded = { cardExpanded = !cardExpanded },
                            onOpenAddMenu = { addMenuExpanded = true },
                        )
                    } else if (controller.selectedNodeDetails != null) {
                        PortraitFloatingObjectCard(
                            controller = controller,
                            expanded = cardExpanded,
                            onToggleExpanded = { cardExpanded = !cardExpanded },
                            onOpenModifiersModal = { activeModalSheet = PortraitModalSheet.Modifiers },
                            onOpenMaterialsModal = { onNavigateToSection(TopSection.Materials) },
                        )
                    }
                }

                // Optional Compact Timeline Bar
                AnimatedVisibility(visible = showCompactTimeline) {
                    CompactTimelineStrip(controller = controller)
                }

                // Bottom Floating Tool Dock (Reference #1)
                Surface(
                    color = NovaColors.PanelBg.copy(alpha = 0.95f),
                    shape = RoundedCornerShape(16.dp),
                    border = BorderStroke(1.dp, NovaColors.PanelBorder),
                    shadowElevation = 10.dp,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 6.dp, vertical = 5.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        NovaDockToolButton(
                            icon = Icons.Rounded.NearMe,
                            label = "Select",
                            active = controller.gizmoMode == GizmoMode.Select,
                            compact = true,
                            onClick = { controller.selectGizmoMode(GizmoMode.Select) },
                        )
                        NovaDockToolButton(
                            icon = Icons.Rounded.OpenWith,
                            label = "Move",
                            active = controller.gizmoMode == GizmoMode.Move,
                            compact = true,
                            onClick = { controller.selectGizmoMode(GizmoMode.Move) },
                        )
                        NovaDockToolButton(
                            icon = Icons.Rounded.RotateRight,
                            label = "Rotate",
                            active = controller.gizmoMode == GizmoMode.Rotate,
                            compact = true,
                            onClick = { controller.selectGizmoMode(GizmoMode.Rotate) },
                        )
                        NovaDockToolButton(
                            icon = Icons.Rounded.AspectRatio,
                            label = "Scale",
                            active = controller.gizmoMode == GizmoMode.Scale,
                            compact = true,
                            onClick = { controller.selectGizmoMode(GizmoMode.Scale) },
                        )
                        Box {
                            NovaDockToolButton(
                                icon = Icons.Rounded.Add,
                                label = "Add",
                                active = addMenuExpanded,
                                compact = true,
                                onClick = { addMenuExpanded = true },
                            )
                            AddObjectDropdownMenu(
                                expanded = addMenuExpanded,
                                onDismiss = { addMenuExpanded = false },
                                onSelectPrimitive = { controller.addPrimitive(it) },
                                onImport3dModel = onImport3dModel,
                            )
                        }
                        NovaDockToolButton(
                            icon = Icons.Rounded.Tune,
                            label = "Edit",
                            active = inEditMode,
                            compact = true,
                            onClick = { controller.toggleInteractionMode() },
                        )
                        NovaDockToolButton(
                            icon = Icons.Rounded.MoreHoriz,
                            label = "More",
                            active = activeModalSheet != PortraitModalSheet.None,
                            compact = true,
                            onClick = {
                                activeModalSheet = if (activeModalSheet == PortraitModalSheet.None) {
                                    PortraitModalSheet.MoreActions
                                } else {
                                    PortraitModalSheet.None
                                }
                            },
                        )
                    }
                }
            }
        }

        // Error banner
        val err = controller.errorMessage
        if (err != null) {
            ErrorOverlayCard(
                message = err,
                onDismiss = { controller.clearError() },
                modifier = Modifier.align(Alignment.Center),
            )
        }
    }
}

@Composable
private fun PortraitMoreToolsGrid(
    controller: ViewportController,
    showCompactTimeline: Boolean,
    onOpenOutliner: () -> Unit,
    onOpenWorld: () -> Unit,
    onOpenModifiers: () -> Unit,
    onToggleTimeline: () -> Unit,
    onNavigate: (TopSection) -> Unit,
) {
    val hasSelection = controller.selectedNodeDetails != null
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            MoreActionTile(Icons.Rounded.AccountTree, "Outliner", Modifier.weight(1f), onClick = onOpenOutliner)
            MoreActionTile(Icons.Rounded.WbSunny, "World", Modifier.weight(1f), onClick = onOpenWorld)
            MoreActionTile(Icons.Rounded.Tune, "Modifiers", Modifier.weight(1f), enabled = hasSelection, onClick = onOpenModifiers)
            MoreActionTile(Icons.Rounded.Timeline, if (showCompactTimeline) "Hide Time" else "Timeline", Modifier.weight(1f), onClick = onToggleTimeline)
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            MoreActionTile(Icons.Rounded.Palette, "Materials", Modifier.weight(1f)) { onNavigate(TopSection.Materials) }
            MoreActionTile(Icons.Rounded.Movie, "Animate", Modifier.weight(1f)) { onNavigate(TopSection.Animation) }
            MoreActionTile(Icons.Rounded.PhotoCamera, "Render", Modifier.weight(1f)) { onNavigate(TopSection.Render) }
            MoreActionTile(Icons.Rounded.ContentCopy, "Duplicate", Modifier.weight(1f), enabled = hasSelection) { controller.duplicateSelected() }
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            MoreActionTile(Icons.Rounded.DeleteOutline, "Delete Obj", Modifier.weight(1f), enabled = hasSelection) { controller.deleteSelected() }
            MoreActionTile(Icons.Rounded.Layers, "Space: ${controller.gizmoSpace.label}", Modifier.weight(1f)) { controller.toggleGizmoSpace() }
            MoreActionTile(Icons.Rounded.NearMe, if (controller.multiSelectEnabled) "Multi: ON" else "Multi: OFF", Modifier.weight(1f)) { controller.toggleMultiSelect() }
            MoreActionTile(Icons.Rounded.Settings, "Settings", Modifier.weight(1f)) { onNavigate(TopSection.Settings) }
        }
    }
}

@Composable
private fun MoreActionTile(
    icon: ImageVector,
    label: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    Surface(
        color = if (enabled) NovaColors.CardBg else NovaColors.CanvasBg,
        shape = RoundedCornerShape(10.dp),
        border = BorderStroke(1.dp, if (enabled) NovaColors.PanelBorder else NovaColors.PanelBorder.copy(alpha = 0.4f)),
        modifier = modifier
            .clip(RoundedCornerShape(10.dp))
            .clickable(enabled = enabled, onClick = onClick),
    ) {
        Column(
            modifier = Modifier.padding(vertical = 8.dp, horizontal = 4.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Icon(
                imageVector = icon,
                contentDescription = label,
                tint = if (enabled) NovaColors.TextPrimary else NovaColors.TextMuted.copy(alpha = 0.4f),
                modifier = Modifier.size(18.dp),
            )
            Text(
                text = label,
                color = if (enabled) NovaColors.TextSecondary else NovaColors.TextMuted.copy(alpha = 0.4f),
                fontSize = 10.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

/**
 * LANDSCAPE WORKSPACE (Reference #3 — 3-Zone Desktop Mobile Workstation):
 * Zone 1: Left Vertical Toolbar (Select, Move, Rotate, Scale, Mode, Vert/Edge/Face, Add, Extrude, Bevel, Cut, Loop Cut)
 * Zone 2: Central Viewport (65-75% width, Orientation Cube, Floating Viewport Actions)
 * Zone 3: Right Collapsible Accordion Properties Panel
 * Zone 4: Bottom Timeline Strip (Play/Pause, Frame counter, Scrubber ruler with keyframes)
 */
@Composable
private fun LandscapeWorkspaceContent(
    controller: ViewportController,
    isLifecycleActive: Boolean,
    isFullscreen: Boolean,
    showSidePanel: Boolean,
    onToggleSidePanel: () -> Unit,
    onImport3dModel: () -> Unit,
) {
    var addMenuExpanded by remember { mutableStateOf(false) }
    var shadingMenuExpanded by remember { mutableStateOf(false) }
    var overlaysMenuExpanded by remember { mutableStateOf(false) }
    var showBottomTimeline by remember { mutableStateOf(true) }

    val inEditMode = controller.editState.interactionMode == InteractionUiMode.Edit
    val es = controller.editState
    val tl = controller.timeline
    val selectedNode = controller.sceneNodes.firstOrNull { it.primarySelected }

    // Collect keyframe frame indices across tracks for timeline markers
    val keyframeFrames = remember(controller.animTracks) {
        controller.animTracks.flatMap { tr -> tr.keyframes.map { it.frame } }.toSet()
    }

    Column(modifier = Modifier.fillMaxSize().background(NovaColors.CanvasBg)) {
        BoxWithConstraints(modifier = Modifier.weight(1f).fillMaxWidth()) {
            val panelWidth = if (maxWidth < 760.dp) 248.dp else 284.dp

            Row(modifier = Modifier.fillMaxSize()) {
                // ZONE 1: Left Vertical Toolbar
                if (!isFullscreen) {
                    Surface(
                        color = NovaColors.PanelBg,
                        border = BorderStroke(1.dp, NovaColors.PanelBorder),
                        modifier = Modifier.width(56.dp).fillMaxHeight(),
                    ) {
                        Column(
                            modifier = Modifier
                                .fillMaxSize()
                                .verticalScroll(rememberScrollState())
                                .padding(vertical = 6.dp, horizontal = 4.dp),
                            horizontalAlignment = Alignment.CenterHorizontally,
                            verticalArrangement = Arrangement.spacedBy(4.dp),
                        ) {
                            NovaDockToolButton(
                                icon = Icons.Rounded.Tune,
                                label = null,
                                active = inEditMode,
                                compact = true,
                                onClick = { controller.toggleInteractionMode() },
                            )
                            HorizontalDivider(color = NovaColors.PanelBorder, modifier = Modifier.padding(vertical = 2.dp))
                            NovaDockToolButton(
                                icon = Icons.Rounded.NearMe,
                                label = null,
                                active = controller.gizmoMode == GizmoMode.Select,
                                compact = true,
                                onClick = { controller.selectGizmoMode(GizmoMode.Select) },
                            )
                            NovaDockToolButton(
                                icon = Icons.Rounded.OpenWith,
                                label = null,
                                active = controller.gizmoMode == GizmoMode.Move,
                                compact = true,
                                onClick = { controller.selectGizmoMode(GizmoMode.Move) },
                            )
                            NovaDockToolButton(
                                icon = Icons.Rounded.RotateRight,
                                label = null,
                                active = controller.gizmoMode == GizmoMode.Rotate,
                                compact = true,
                                onClick = { controller.selectGizmoMode(GizmoMode.Rotate) },
                            )
                            NovaDockToolButton(
                                icon = Icons.Rounded.AspectRatio,
                                label = null,
                                active = controller.gizmoMode == GizmoMode.Scale,
                                compact = true,
                                onClick = { controller.selectGizmoMode(GizmoMode.Scale) },
                            )
                            HorizontalDivider(color = NovaColors.PanelBorder, modifier = Modifier.padding(vertical = 2.dp))
                            // Vertex / Edge / Face selection buttons
                            EditSelectUiMode.entries.forEach { sm ->
                                val icon = when (sm) {
                                    EditSelectUiMode.Vertex -> Icons.Rounded.FiberManualRecord
                                    EditSelectUiMode.Edge -> Icons.Rounded.ShowChart
                                    EditSelectUiMode.Face -> Icons.Rounded.CropSquare
                                }
                                NovaDockToolButton(
                                    icon = icon,
                                    label = null,
                                    active = inEditMode && es.selectMode == sm,
                                    compact = true,
                                    onClick = {
                                        if (!inEditMode) {
                                            controller.setInteractionMode(InteractionUiMode.Edit)
                                        }
                                        controller.setEditSelectMode(sm)
                                    },
                                )
                            }
                            HorizontalDivider(color = NovaColors.PanelBorder, modifier = Modifier.padding(vertical = 2.dp))
                            Box {
                                NovaDockToolButton(
                                    icon = Icons.Rounded.Add,
                                    label = "Add",
                                    active = addMenuExpanded,
                                    compact = true,
                                    onClick = { addMenuExpanded = true },
                                )
                                AddObjectDropdownMenu(
                                    expanded = addMenuExpanded,
                                    onDismiss = { addMenuExpanded = false },
                                    onSelectPrimitive = { controller.addPrimitive(it) },
                                    onImport3dModel = onImport3dModel,
                                )
                            }
                            if (inEditMode) {
                                NovaDockToolButton(
                                    icon = Icons.Rounded.CallMade,
                                    label = "Extrude",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editExtrude() },
                                )
                                NovaDockToolButton(
                                    icon = Icons.Rounded.RoundedCorner,
                                    label = "Bevel",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editBevel() },
                                )
                                NovaDockToolButton(
                                    icon = Icons.Rounded.CenterFocusStrong,
                                    label = "Inset",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editInset() },
                                )
                                NovaDockToolButton(
                                    icon = Icons.Rounded.ContentCut,
                                    label = "Loop",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editLoopCut() },
                                )
                                NovaDockToolButton(
                                    icon = Icons.Rounded.Grid4x4,
                                    label = "Subdiv",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editSubdivide() },
                                )
                                NovaDockToolButton(
                                    icon = Icons.Rounded.CallMerge,
                                    label = "Merge",
                                    active = false,
                                    compact = true,
                                    onClick = { controller.editMerge() },
                                )
                            }
                        }
                    }
                }

                // ZONE 2: Center 3D Viewport
                Box(modifier = Modifier.weight(1f).fillMaxHeight().background(NovaColors.CanvasBg)) {
                    ViewportSurface(
                        controller = controller,
                        isActive = isLifecycleActive,
                        modifier = Modifier.fillMaxSize(),
                    )

                    // Top-Left HUD (Perspective View, Verts/Faces, FPS)
                    Column(
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .padding(10.dp),
                        verticalArrangement = Arrangement.spacedBy(2.dp),
                    ) {
                        Text(
                            text = "${controller.projectionMode.label} View · ${controller.activeBackendName} (${controller.fps} FPS)",
                            color = NovaColors.TextSecondary,
                            fontSize = 11.sp,
                            fontWeight = FontWeight.Medium,
                        )
                        if (inEditMode) {
                            Text(
                                text = "Verts: ${es.totalVerts}  |  Faces: ${es.totalFaces}  |  Sel: ${es.activeSelectedCount}",
                                color = NovaColors.AccentVioletGlow,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                            )
                        } else {
                            Text(
                                text = "Sel: ${selectedNode?.name ?: "None"} · Space: ${controller.gizmoSpace.label}",
                                color = NovaColors.TextMuted,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                            )
                        }
                    }

                    // Top-Right Orientation Cube & Floating Viewport Overlay Bar
                    Column(
                        modifier = Modifier
                            .align(Alignment.TopEnd)
                            .padding(10.dp),
                        horizontalAlignment = Alignment.End,
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        OrientationCubeWidget(
                            yawDeg = controller.cameraYawDeg,
                            pitchDeg = controller.cameraPitchDeg,
                            onSelectViewPreset = { controller.setStandardView(it) },
                        )

                        if (!isFullscreen) {
                            Surface(
                                color = NovaColors.PanelBg.copy(alpha = 0.88f),
                                shape = RoundedCornerShape(12.dp),
                                border = BorderStroke(1.dp, NovaColors.PanelBorder),
                            ) {
                                Column(
                                    modifier = Modifier.padding(3.dp),
                                    verticalArrangement = Arrangement.spacedBy(2.dp),
                                    horizontalAlignment = Alignment.CenterHorizontally,
                                ) {
                                    Box {
                                        ViewportActionIcon(
                                            icon = Icons.Rounded.Tonality,
                                            contentDescription = "Shading",
                                            active = controller.shadingMode != ShadingUiMode.Solid,
                                            onClick = { shadingMenuExpanded = true },
                                        )
                                        DropdownMenu(
                                            expanded = shadingMenuExpanded,
                                            onDismissRequest = { shadingMenuExpanded = false },
                                            containerColor = NovaColors.PanelBg,
                                        ) {
                                            ShadingUiMode.entries.forEach { mode ->
                                                DropdownMenuItem(
                                                    text = {
                                                        Text(
                                                            mode.label,
                                                            color = if (controller.shadingMode == mode) NovaColors.AccentVioletGlow else NovaColors.TextPrimary,
                                                            fontSize = 12.sp,
                                                        )
                                                    },
                                                    onClick = {
                                                        shadingMenuExpanded = false
                                                        controller.setShadingMode(mode)
                                                    },
                                                )
                                            }
                                        }
                                    }

                                    Box {
                                        ViewportActionIcon(
                                            icon = Icons.Rounded.GridOn,
                                            contentDescription = "Overlays",
                                            active = controller.showGrid || controller.showAxes,
                                            onClick = { overlaysMenuExpanded = true },
                                        )
                                        DropdownMenu(
                                            expanded = overlaysMenuExpanded,
                                            onDismissRequest = { overlaysMenuExpanded = false },
                                            containerColor = NovaColors.PanelBg,
                                        ) {
                                            DropdownMenuItem(
                                                text = { Text("Grid: ${if (controller.showGrid) "ON" else "OFF"}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                                onClick = { controller.toggleGrid() },
                                            )
                                            DropdownMenuItem(
                                                text = { Text("Axes: ${if (controller.showAxes) "ON" else "OFF"}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                                onClick = { controller.toggleAxes() },
                                            )
                                            DropdownMenuItem(
                                                text = { Text("Space: ${controller.gizmoSpace.label}", color = NovaColors.TextPrimary, fontSize = 12.sp) },
                                                onClick = { controller.toggleGizmoSpace() },
                                            )
                                        }
                                    }

                                    ViewportActionIcon(Icons.Rounded.CenterFocusWeak, "Frame All") { controller.frameAll() }
                                    ViewportActionIcon(Icons.Rounded.FilterCenterFocus, "Focus Selected") { controller.focusSelected() }
                                    ViewportActionIcon(
                                        icon = Icons.Rounded.Videocam,
                                        contentDescription = "Cam View",
                                        active = tl.activeCameraNodeId != 0L,
                                        onClick = { controller.toggleCameraView() },
                                    )
                                    ViewportActionIcon(
                                        icon = Icons.Rounded.ViewInAr,
                                        contentDescription = "Persp/Ortho",
                                        active = controller.projectionMode.code == 1,
                                        onClick = { controller.toggleProjection() },
                                    )
                                    ViewportActionIcon(
                                        icon = Icons.Rounded.Timeline,
                                        contentDescription = "Toggle Timeline",
                                        active = showBottomTimeline,
                                        onClick = { showBottomTimeline = !showBottomTimeline },
                                    )
                                }
                            }
                        }
                    }

                    // Transient status toast
                    val banner = controller.statusBanner
                    if (banner != null) {
                        Surface(
                            color = NovaColors.PanelBg.copy(alpha = 0.94f),
                            shape = RoundedCornerShape(10.dp),
                            border = BorderStroke(1.dp, NovaColors.AccentViolet),
                            modifier = Modifier.align(Alignment.TopCenter).padding(top = 12.dp),
                        ) {
                            Text(
                                text = banner,
                                color = NovaColors.TextPrimary,
                                fontSize = 12.sp,
                                modifier = Modifier.padding(horizontal = 14.dp, vertical = 7.dp),
                            )
                        }
                    }

                    val err = controller.errorMessage
                    if (err != null) {
                        ErrorOverlayCard(
                            message = err,
                            onDismiss = { controller.clearError() },
                            modifier = Modifier.align(Alignment.Center),
                        )
                    }
                }

                // ZONE 3: Right Collapsible Properties Panel + Handle
                if (!isFullscreen) {
                    // Collapse / Expand handle tab
                    Box(
                        modifier = Modifier
                            .width(16.dp)
                            .fillMaxHeight()
                            .background(NovaColors.CanvasBg)
                            .clickable(onClick = onToggleSidePanel),
                        contentAlignment = Alignment.Center,
                    ) {
                        Surface(
                            color = NovaColors.PanelBg,
                            shape = RoundedCornerShape(topStart = 6.dp, bottomStart = 6.dp),
                            border = BorderStroke(1.dp, NovaColors.PanelBorder),
                            modifier = Modifier.width(15.dp).height(44.dp),
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                Icon(
                                    imageVector = if (showSidePanel) Icons.Rounded.ChevronRight else Icons.Rounded.ChevronLeft,
                                    contentDescription = "Toggle Panel",
                                    tint = NovaColors.TextSecondary,
                                    modifier = Modifier.size(14.dp),
                                )
                            }
                        }
                    }

                    if (showSidePanel) {
                        LandscapeAccordionPropertiesPanel(
                            controller = controller,
                            modifier = Modifier
                                .width(panelWidth)
                                .fillMaxHeight(),
                        )
                    }
                }
            }
        }

        // ZONE 4: Bottom Timeline Scrubber Strip
        if (!isFullscreen && showBottomTimeline) {
            LandscapeTimelineStrip(
                currentFrame = tl.currentFrame,
                startFrame = tl.startFrame,
                endFrame = tl.endFrame,
                isPlaying = tl.playing,
                autoKey = tl.autoKey,
                keyframeFrames = keyframeFrames,
                onRewind = { controller.rewindToStart() },
                onTogglePlay = { controller.togglePlaying() },
                onStepNextKey = { controller.stepKeyframe(1) },
                onAddKey = { controller.keySelectedNodeTrs(7) },
                onToggleAutoKey = { controller.updateTimelineConfig(autoKey = !tl.autoKey) },
                onScrubFrame = { controller.setCurrentFrame(it) },
            )
        }
    }
}

@Composable
private fun AddObjectDropdownMenu(
    expanded: Boolean,
    onDismiss: () -> Unit,
    onSelectPrimitive: (PrimitiveUiType) -> Unit,
    onImport3dModel: () -> Unit,
) {
    DropdownMenu(
        expanded = expanded,
        onDismissRequest = onDismiss,
        containerColor = NovaColors.PanelBg,
    ) {
        PrimitiveUiType.entries.forEach { prim ->
            DropdownMenuItem(
                text = {
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Surface(
                            color = NovaColors.CardBg,
                            shape = RoundedCornerShape(4.dp),
                        ) {
                            Text(
                                text = prim.group.uppercase(Locale.US),
                                color = NovaColors.TextMuted,
                                fontSize = 9.sp,
                                fontFamily = FontFamily.Monospace,
                                modifier = Modifier.padding(horizontal = 5.dp, vertical = 2.dp),
                            )
                        }
                        Text(
                            text = prim.label,
                            color = NovaColors.TextPrimary,
                            fontSize = 13.sp,
                        )
                    }
                },
                onClick = {
                    onDismiss()
                    onSelectPrimitive(prim)
                },
            )
        }
        HorizontalDivider(color = NovaColors.PanelBorder)
        DropdownMenuItem(
            text = {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(
                        text = "IMPORT",
                        color = NovaColors.AccentCyan,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        text = "3D File (GLB / glTF / OBJ)",
                        color = NovaColors.TextPrimary,
                        fontSize = 13.sp,
                    )
                }
            },
            onClick = {
                onDismiss()
                onImport3dModel()
            },
        )
    }
}

@Composable
private fun CompactTimelineStrip(controller: ViewportController) {
    val tl = controller.timeline
    val sF = tl.startFrame.toFloat()
    val eF = tl.endFrame.toFloat().coerceAtLeast(sF + 1f)
    Surface(
        color = NovaColors.PanelBg.copy(alpha = 0.95f),
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp, vertical = 5.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            MiniActionChip("|◀", false) { controller.rewindToStart() }
            MiniActionChip(if (tl.playing) "⏸" else "▶", tl.playing) { controller.togglePlaying() }
            MiniActionChip("+ Key ◆", false) { controller.keySelectedNodeTrs(7) }
            Text(
                text = "F${tl.currentFrame.roundToInt()}/${tl.endFrame}",
                color = NovaColors.TextPrimary,
                fontSize = 10.sp,
                fontFamily = FontFamily.Monospace,
            )
            androidx.compose.material3.Slider(
                value = tl.currentFrame.coerceIn(sF, eF),
                onValueChange = { controller.setCurrentFrame(it.roundToInt().toFloat()) },
                valueRange = sF..eF,
                colors = androidx.compose.material3.SliderDefaults.colors(
                    thumbColor = NovaColors.AccentViolet,
                    activeTrackColor = NovaColors.AccentViolet,
                    inactiveTrackColor = NovaColors.PanelBorder,
                ),
                modifier = Modifier.weight(1f).height(24.dp),
            )
            MiniActionChip(if (tl.autoKey) "Auto" else "Man", tl.autoKey) {
                controller.updateTimelineConfig(autoKey = !tl.autoKey)
            }
        }
    }
}

@Composable
private fun ErrorOverlayCard(
    message: String,
    onDismiss: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        color = NovaColors.ErrorRed.copy(alpha = 0.94f),
        shape = RoundedCornerShape(12.dp),
        modifier = modifier
            .padding(20.dp)
            .clickable(onClick = onDismiss),
    ) {
        Text(
            text = "$message\n(Tap to dismiss)",
            color = Color.White,
            fontSize = 13.sp,
            modifier = Modifier.padding(14.dp),
        )
    }
}

@Composable
private fun SettingsScreen(controller: ViewportController) {
    val context = LocalContext.current
    var caps by remember { mutableStateOf<DeviceRenderCapabilities?>(null) }
    LaunchedEffect(Unit) {
        caps = withContext(Dispatchers.IO) { OffscreenRenderEngine.probeCapabilities(context) }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            NovaBrandMark(size = 28.dp)
            Column {
                Text(
                    text = "Settings & Engine Diagnostics",
                    color = NovaColors.TextPrimary,
                    fontSize = 17.sp,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    text = "NOVA 3D Studio · Vulkan / Filament Runtime",
                    color = NovaColors.TextSecondary,
                    fontSize = 11.sp,
                )
            }
        }

        // Viewport & Performance Category
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(
                    text = "VIEWPORT & PERFORMANCE",
                    color = NovaColors.AccentVioletGlow,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.6.sp,
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "Adaptive Viewport Resolution",
                            color = NovaColors.TextPrimary,
                            fontSize = 13.sp,
                            fontWeight = FontWeight.Medium,
                        )
                        Text(
                            text = "Dynamic resolution scaling keeps viewport responsive during dense mesh edits.",
                            color = NovaColors.TextSecondary,
                            fontSize = 11.sp,
                        )
                    }
                    Switch(
                        checked = controller.adaptiveViewportQuality,
                        onCheckedChange = { controller.setAdaptiveViewport(it) },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = Color.White,
                            checkedTrackColor = NovaColors.AccentViolet,
                        ),
                    )
                }
                HorizontalDivider(color = NovaColors.PanelBorder)
                SettingRow("Active Render Backend", controller.activeBackendName)
                SettingRow("Projection Mode", controller.projectionMode.label)
                SettingRow("Shading Mode", controller.shadingMode.label)
                SettingRow("Gizmo Mode / Space", "${controller.gizmoMode.label} (${controller.gizmoSpace.label})")
                SettingRow("Grid / Axes Overlays", "${controller.showGrid} / ${controller.showAxes}")
            }
        }

        // Scene & Hardware Category
        Surface(
            color = NovaColors.PanelBg,
            shape = RoundedCornerShape(12.dp),
            border = BorderStroke(1.dp, NovaColors.PanelBorder),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(
                modifier = Modifier.padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(
                    text = "HARDWARE & SCENE TELEMETRY",
                    color = NovaColors.AccentCyan,
                    fontSize = 11.sp,
                    fontWeight = FontWeight.Bold,
                    letterSpacing = 0.6.sp,
                )
                SettingRow("Active Project", controller.projectTitle)
                SettingRow("Scene Nodes (Rust Core)", controller.sceneNodes.size.toString())
                if (caps != null) {
                    val c = caps!!
                    SettingRow("Device Memory", "${c.availableRamMb} / ${c.totalRamMb} MiB free")
                    SettingRow("H.264 Encoder", c.avcEncoderName ?: "Software / Unavailable")
                    SettingRow("Max Video Encode Size", "${c.maxVideoWidth} x ${c.maxVideoHeight}")
                }
                SettingRow("Last Native Error", controller.errorMessage ?: "None")
            }
        }
    }
}

@Composable
private fun SettingRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(text = label, color = NovaColors.TextSecondary, fontSize = 12.sp)
        Text(
            text = value,
            color = NovaColors.TextPrimary,
            fontSize = 12.sp,
            fontFamily = FontFamily.Monospace,
        )
    }
}
