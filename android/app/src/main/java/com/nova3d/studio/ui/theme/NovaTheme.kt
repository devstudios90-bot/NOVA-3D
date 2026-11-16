package com.nova3d.studio.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

object NovaColors {
    // Master Dark Creative Studio Palette
    val CanvasBg = Color(0xFF0D0E12)        // Deep charcoal / near-black
    val PanelBg = Color(0xFF14151E)         // Dark graphite
    val CardBg = Color(0xFF1A1C27)          // Elevated card charcoal
    val CardBgElevated = Color(0xFF222433)  // Interactive surface
    val PanelBorder = Color(0xFF27293D)     // Subtle dark-gray border
    val BorderBright = Color(0xFF383B54)

    // Primary Electric Violet & Secondary Cool Cyan
    val AccentViolet = Color(0xFF7B5CFF)
    val AccentVioletSoft = Color(0xFF251D4A)
    val AccentVioletGlow = Color(0xFF947BFF)
    val AccentCyan = Color(0xFF38BDF8)

    // Backward-compatible alias mapped to primary Electric Violet
    val AccentBlue = AccentViolet
    val AccentBlueSoft = AccentVioletSoft

    // Typography & Neutrals
    val TextPrimary = Color(0xFFF3F4F6)
    val TextSecondary = Color(0xFF94A3B8)
    val TextMuted = Color(0xFF64748B)

    // Semantic Status
    val SuccessGreen = Color(0xFF22C55E)
    val WarningAmber = Color(0xFFF59E0B)
    val ErrorRed = Color(0xFFEF4444)

    // Axis Colors (X Red, Y Green, Z Blue/Cyan)
    val AxisX = Color(0xFFEF4444)
    val AxisY = Color(0xFF22C55E)
    val AxisZ = Color(0xFF3B82F6)
}

private val NovaDarkScheme = darkColorScheme(
    primary = NovaColors.AccentViolet,
    onPrimary = Color.White,
    secondary = NovaColors.AccentCyan,
    onSecondary = Color(0xFF09101A),
    tertiary = NovaColors.CardBgElevated,
    onTertiary = NovaColors.TextPrimary,
    background = NovaColors.CanvasBg,
    onBackground = NovaColors.TextPrimary,
    surface = NovaColors.PanelBg,
    onSurface = NovaColors.TextPrimary,
    surfaceVariant = NovaColors.CardBg,
    onSurfaceVariant = NovaColors.TextSecondary,
    outline = NovaColors.PanelBorder,
    error = NovaColors.ErrorRed,
    onError = Color.White,
)

private val NovaTypography = Typography(
    titleLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 18.sp,
        letterSpacing = (-0.2).sp,
    ),
    titleMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 14.sp,
        letterSpacing = 0.1.sp,
    ),
    labelLarge = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 12.sp,
        letterSpacing = 0.2.sp,
    ),
    labelMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 11.sp,
    ),
    labelSmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 10.sp,
    ),
    bodyMedium = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 13.sp,
    ),
    bodySmall = TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 11.sp,
    ),
)

@Composable
fun NovaTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = NovaDarkScheme,
        typography = NovaTypography,
        content = content,
    )
}
