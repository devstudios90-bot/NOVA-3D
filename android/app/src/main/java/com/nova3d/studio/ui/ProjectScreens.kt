package com.nova3d.studio.ui

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.FolderOpen
import androidx.compose.material.icons.rounded.OpenInNew
import androidx.compose.material.icons.rounded.ViewInAr
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.nova3d.studio.storage.ProjectSummary
import com.nova3d.studio.ui.components.NovaBrandMark
import com.nova3d.studio.ui.theme.NovaColors
import com.nova3d.studio.viewport.ViewportController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.text.DateFormat
import java.util.Date

@Composable
fun HomeScreen(
    controller: ViewportController,
    projects: List<ProjectSummary>,
    onNewEmpty: () -> Unit,
    onNewStarter: () -> Unit,
    onOpenWorkspace: () -> Unit,
    onOpenProject: (ProjectSummary) -> Unit,
    onImportNova3d: () -> Unit,
) {
    BoxWithConstraints(
        modifier = Modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg),
    ) {
        val isWide = maxWidth >= 680.dp || (maxWidth > maxHeight && maxWidth >= 520.dp)

        if (isWide) {
            Row(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp),
                horizontalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Column(
                    modifier = Modifier
                        .weight(0.46f)
                        .fillMaxHeight()
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    HomeBrandingBanner()
                    ActiveWorkspaceCard(controller, onOpenWorkspace)
                    CreateProjectTemplatesCard(onNewStarter, onNewEmpty, onImportNova3d)
                }
                Column(
                    modifier = Modifier
                        .weight(0.54f)
                        .fillMaxHeight(),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        text = "Recent Projects (${projects.size})",
                        color = NovaColors.TextPrimary,
                        fontSize = 15.sp,
                        fontWeight = FontWeight.SemiBold,
                    )
                    RecentProjectsList(
                        controller = controller,
                        projects = projects,
                        onOpenProject = onOpenProject,
                        modifier = Modifier.weight(1f).fillMaxWidth(),
                    )
                }
            }
        } else {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(14.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                HomeBrandingBanner()
                ActiveWorkspaceCard(controller, onOpenWorkspace)
                CreateProjectTemplatesCard(onNewStarter, onNewEmpty, onImportNova3d)
                Text(
                    text = "Recent Projects (${projects.size})",
                    color = NovaColors.TextPrimary,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                if (projects.isEmpty()) {
                    EmptyProjectsCard()
                } else {
                    projects.take(6).forEach { summary ->
                        ProjectListRow(
                            summary = summary,
                            isActive = controller.currentFileName == summary.fileName,
                            onOpen = { onOpenProject(summary) },
                            onExport = null,
                            onDelete = null,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun HomeBrandingBanner() {
    Row(
        modifier = Modifier.fillMaxWidth().padding(vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        NovaBrandMark(size = 38.dp)
        Column {
            Text(
                text = buildAnnotatedString {
                    withStyle(SpanStyle(color = NovaColors.TextPrimary, fontWeight = FontWeight.Bold)) {
                        append("NOVA ")
                    }
                    withStyle(SpanStyle(color = NovaColors.AccentVioletGlow, fontWeight = FontWeight.Bold)) {
                        append("3D")
                    }
                },
                fontSize = 20.sp,
                letterSpacing = 0.5.sp,
            )
            Text(
                text = "Mobile 3D Modeling, Animation & PBR Workstation",
                color = NovaColors.TextSecondary,
                fontSize = 12.sp,
            )
        }
    }
}

@Composable
private fun ActiveWorkspaceCard(
    controller: ViewportController,
    onOpenWorkspace: () -> Unit,
) {
    Surface(
        color = NovaColors.PanelBg,
        shape = RoundedCornerShape(14.dp),
        border = BorderStroke(1.dp, NovaColors.AccentViolet.copy(alpha = 0.45f)),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Box(
            modifier = Modifier.background(
                Brush.horizontalGradient(
                    colors = listOf(
                        NovaColors.AccentVioletSoft.copy(alpha = 0.55f),
                        NovaColors.PanelBg,
                    ),
                ),
            ),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth().padding(14.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(
                        text = "CURRENTLY LOADED SCENE",
                        color = NovaColors.AccentCyan,
                        fontSize = 10.sp,
                        fontWeight = FontWeight.Bold,
                        letterSpacing = 0.7.sp,
                    )
                    Text(
                        text = controller.projectTitle + if (controller.hasUnsavedChanges) " • unsaved" else "",
                        color = NovaColors.TextPrimary,
                        fontSize = 16.sp,
                        fontWeight = FontWeight.Bold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "${controller.sceneNodes.size} objects · ${controller.activeBackendName} Engine",
                        color = NovaColors.TextSecondary,
                        fontSize = 11.sp,
                    )
                }
                Spacer(Modifier.width(10.dp))
                Button(
                    onClick = onOpenWorkspace,
                    colors = ButtonDefaults.buttonColors(containerColor = NovaColors.AccentViolet),
                    shape = RoundedCornerShape(10.dp),
                ) {
                    Icon(Icons.Rounded.ViewInAr, contentDescription = null, modifier = Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Open Editor", fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
                }
            }
        }
    }
}

@Composable
private fun CreateProjectTemplatesCard(
    onNewStarter: () -> Unit,
    onNewEmpty: () -> Unit,
    onImportNova3d: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(
            text = "New Project & Templates",
            color = NovaColors.TextPrimary,
            fontSize = 14.sp,
            fontWeight = FontWeight.SemiBold,
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            QuickStartCard(
                title = "Studio Starter",
                subtitle = "Cube + 3-pt lights",
                icon = Icons.Rounded.Add,
                accent = NovaColors.AccentViolet,
                highlighted = true,
                modifier = Modifier.weight(1f),
                onClick = onNewStarter,
            )
            QuickStartCard(
                title = "Empty Scene",
                subtitle = "Blank 3D stage",
                icon = Icons.Rounded.ViewInAr,
                accent = NovaColors.AccentCyan,
                highlighted = false,
                modifier = Modifier.weight(1f),
                onClick = onNewEmpty,
            )
            QuickStartCard(
                title = "Import File",
                subtitle = "Open .nova3d",
                icon = Icons.Rounded.FolderOpen,
                accent = NovaColors.SuccessGreen,
                highlighted = false,
                modifier = Modifier.weight(1f),
                onClick = onImportNova3d,
            )
        }
    }
}

@Composable
private fun RecentProjectsList(
    controller: ViewportController,
    projects: List<ProjectSummary>,
    onOpenProject: (ProjectSummary) -> Unit,
    modifier: Modifier = Modifier,
) {
    if (projects.isEmpty()) {
        Box(modifier = modifier, contentAlignment = Alignment.Center) {
            EmptyProjectsCard()
        }
    } else {
        LazyColumn(
            modifier = modifier,
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(projects, key = { it.fileName }) { summary ->
                ProjectListRow(
                    summary = summary,
                    isActive = controller.currentFileName == summary.fileName,
                    onOpen = { onOpenProject(summary) },
                    onExport = null,
                    onDelete = null,
                )
            }
        }
    }
}

@Composable
private fun EmptyProjectsCard() {
    Surface(
        color = NovaColors.PanelBg,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, NovaColors.PanelBorder),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(
            text = "No saved projects yet. Save your active scene from the top header or Projects workspace to build your library.",
            color = NovaColors.TextSecondary,
            fontSize = 12.sp,
            modifier = Modifier.padding(16.dp),
        )
    }
}

@Composable
fun ProjectsScreen(
    controller: ViewportController,
    projects: List<ProjectSummary>,
    onSaveCurrent: () -> Unit,
    onSaveAs: () -> Unit,
    onOpenProject: (ProjectSummary) -> Unit,
    onDeleteProject: (ProjectSummary) -> Unit,
    onImportNova3d: () -> Unit,
    onExportNova3d: () -> Unit,
    onImport3dModel: () -> Unit,
    onExportGlb: () -> Unit,
    onExportObj: () -> Unit,
) {
    var pendingDelete by remember { mutableStateOf<ProjectSummary?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(NovaColors.CanvasBg)
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
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
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = controller.projectTitle + if (controller.hasUnsavedChanges) " (Unsaved)" else "",
                            color = NovaColors.TextPrimary,
                            fontSize = 16.sp,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            text = controller.currentFileName ?: "Not yet saved to project library",
                            color = NovaColors.TextMuted,
                            fontSize = 11.sp,
                            fontFamily = FontFamily.Monospace,
                        )
                    }
                }

                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    Button(
                        onClick = onSaveCurrent,
                        colors = ButtonDefaults.buttonColors(containerColor = NovaColors.AccentViolet),
                        shape = RoundedCornerShape(8.dp),
                    ) {
                        Text("Save", fontSize = 12.sp)
                    }
                    OutlinedButton(onClick = onSaveAs, shape = RoundedCornerShape(8.dp)) {
                        Text("Save As", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                    OutlinedButton(onClick = onExportNova3d, shape = RoundedCornerShape(8.dp)) {
                        Text("Export .nova3d", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                    OutlinedButton(onClick = onImportNova3d, shape = RoundedCornerShape(8.dp)) {
                        Text("Import .nova3d", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                }
                Row(
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    OutlinedButton(onClick = onImport3dModel, shape = RoundedCornerShape(8.dp)) {
                        Text("Import 3D (GLB/glTF/OBJ)", fontSize = 12.sp, color = NovaColors.AccentCyan)
                    }
                    OutlinedButton(onClick = onExportGlb, shape = RoundedCornerShape(8.dp)) {
                        Text("Export GLB", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                    OutlinedButton(onClick = onExportObj, shape = RoundedCornerShape(8.dp)) {
                        Text("Export OBJ", fontSize = 12.sp, color = NovaColors.TextPrimary)
                    }
                }
            }
        }

        Text(
            text = "Saved .nova3d Projects (${projects.size})",
            color = NovaColors.TextPrimary,
            fontSize = 14.sp,
            fontWeight = FontWeight.SemiBold,
        )

        if (projects.isEmpty()) {
            Box(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                contentAlignment = Alignment.Center,
            ) {
                Text(
                    text = "Tap Save or Save As above to write your first .nova3d file.",
                    color = NovaColors.TextMuted,
                    fontSize = 13.sp,
                )
            }
        } else {
            LazyColumn(
                modifier = Modifier.weight(1f).fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(projects, key = { it.fileName }) { summary ->
                    ProjectListRow(
                        summary = summary,
                        isActive = controller.currentFileName == summary.fileName,
                        onOpen = { onOpenProject(summary) },
                        onExport = null,
                        onDelete = { pendingDelete = summary },
                    )
                }
            }
        }
    }

    val delTarget = pendingDelete
    if (delTarget != null) {
        AlertDialog(
            onDismissRequest = { pendingDelete = null },
            containerColor = NovaColors.PanelBg,
            title = { Text("Delete Project?", color = NovaColors.TextPrimary, fontSize = 16.sp) },
            text = {
                Text(
                    text = "Permanently delete \"${delTarget.title}\" (${delTarget.fileName})?",
                    color = NovaColors.TextSecondary,
                    fontSize = 13.sp,
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        pendingDelete = null
                        onDeleteProject(delTarget)
                    },
                ) {
                    Text("Delete", color = NovaColors.ErrorRed)
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingDelete = null }) {
                    Text("Cancel", color = NovaColors.TextSecondary)
                }
            },
        )
    }
}

@Composable
private fun QuickStartCard(
    title: String,
    subtitle: String,
    icon: ImageVector,
    accent: Color,
    highlighted: Boolean,
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
) {
    Surface(
        color = if (highlighted) NovaColors.AccentVioletSoft else NovaColors.PanelBg,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(1.dp, if (highlighted) NovaColors.AccentViolet else accent.copy(alpha = 0.45f)),
        modifier = modifier
            .clip(RoundedCornerShape(12.dp))
            .clickable(onClick = onClick),
    ) {
        Column(
            modifier = Modifier.padding(11.dp),
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            Box(
                modifier = Modifier
                    .size(28.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .background(accent.copy(alpha = 0.2f)),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = icon,
                    contentDescription = title,
                    tint = if (highlighted) Color.White else accent,
                    modifier = Modifier.size(16.dp),
                )
            }
            Text(
                text = title,
                color = NovaColors.TextPrimary,
                fontSize = 12.sp,
                fontWeight = FontWeight.SemiBold,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                text = subtitle,
                color = NovaColors.TextMuted,
                fontSize = 10.sp,
                lineHeight = 13.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
    }
}

@Composable
private fun ProjectListRow(
    summary: ProjectSummary,
    isActive: Boolean,
    onOpen: () -> Unit,
    onExport: (() -> Unit)?,
    onDelete: (() -> Unit)?,
) {
    val thumbBitmap by produceState<Bitmap?>(initialValue = null, summary.thumbnailPath, summary.modifiedAtMillis) {
        val path = summary.thumbnailPath
        value = if (path != null) {
            withContext(Dispatchers.IO) {
                runCatching { BitmapFactory.decodeFile(path) }.getOrNull()
            }
        } else {
            null
        }
    }

    val dateStr = remember(summary.modifiedAtMillis) {
        DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT)
            .format(Date(summary.modifiedAtMillis))
    }

    Surface(
        color = NovaColors.PanelBg,
        shape = RoundedCornerShape(12.dp),
        border = BorderStroke(
            1.dp,
            if (isActive) NovaColors.AccentViolet else NovaColors.PanelBorder,
        ),
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .clickable(onClick = onOpen),
    ) {
        Row(
            modifier = Modifier.padding(10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Box(
                modifier = Modifier
                    .size(width = 74.dp, height = 48.dp)
                    .clip(RoundedCornerShape(8.dp))
                    .background(NovaColors.CanvasBg),
                contentAlignment = Alignment.Center,
            ) {
                val bmp = thumbBitmap
                if (bmp != null) {
                    Image(
                        bitmap = bmp.asImageBitmap(),
                        contentDescription = summary.title,
                        contentScale = ContentScale.Crop,
                        modifier = Modifier.fillMaxSize(),
                    )
                } else {
                    Text(
                        text = ".nova3d",
                        color = NovaColors.TextMuted,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                    )
                }
            }

            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(3.dp),
            ) {
                Text(
                    text = summary.title,
                    color = NovaColors.TextPrimary,
                    fontSize = 14.sp,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    text = String.format(
                        java.util.Locale.US,
                        "%d objs · %.1f KB · %s",
                        summary.nodeCount,
                        (summary.sizeBytes / 1024.0).coerceAtLeast(0.1),
                        dateStr,
                    ),
                    color = NovaColors.TextMuted,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }

            Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                Surface(
                    color = if (isActive) NovaColors.AccentViolet else NovaColors.CardBg,
                    shape = RoundedCornerShape(7.dp),
                    modifier = Modifier.clip(RoundedCornerShape(7.dp)).clickable(onClick = onOpen),
                ) {
                    Row(
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 5.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.OpenInNew,
                            contentDescription = "Open",
                            tint = Color.White,
                            modifier = Modifier.size(13.dp),
                        )
                        Text("Open", color = Color.White, fontSize = 10.sp, fontWeight = FontWeight.SemiBold)
                    }
                }
                if (onExport != null) {
                    MiniActionChip(label = "Export", active = false, onClick = onExport)
                }
                if (onDelete != null) {
                    Box(
                        modifier = Modifier
                            .size(28.dp)
                            .clip(RoundedCornerShape(7.dp))
                            .background(NovaColors.CardBg)
                            .clickable(onClick = onDelete),
                        contentAlignment = Alignment.Center,
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.DeleteOutline,
                            contentDescription = "Delete",
                            tint = NovaColors.ErrorRed,
                            modifier = Modifier.size(16.dp),
                        )
                    }
                }
            }
        }
    }
}
