import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
}

val novaRootDir = rootProject.projectDir.parentFile.absolutePath.replace("\\", "/")
val rustCoreDir = rootProject.projectDir.parentFile.resolve("rust/nova_core")
val ndkVer = "30.0.16248370"

val localProps = Properties().apply {
    val f = rootProject.file("local.properties")
    if (f.exists()) f.inputStream().use { load(it) }
}
val sdkDirPath: String? = localProps.getProperty("sdk.dir")
    ?: System.getenv("ANDROID_HOME")
    ?: System.getenv("ANDROID_SDK_ROOT")

val buildRustCore = tasks.register<Exec>("buildRustCore") {
    group = "build"
    description = "Compiles the Rust nova_core static libraries for Android ABIs via cargo-ndk."
    workingDir = rustCoreDir
    if (sdkDirPath != null) {
        environment("ANDROID_NDK_HOME", "$sdkDirPath/ndk/$ndkVer")
    }
    commandLine(
        "cargo", "ndk",
        "-t", "arm64-v8a",
        "-t", "armeabi-v7a",
        "-t", "x86_64",
        "-P", "26",
        "build", "--release",
    )
}

android {
    namespace = "com.nova3d.studio"
    compileSdk = 35
    ndkVersion = ndkVer

    defaultConfig {
        applicationId = "com.nova3d.studio"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0-m1"

        ndk {
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }

        externalNativeBuild {
            cmake {
                cppFlags("-std=c++20", "-fexceptions")
                arguments(
                    "-DANDROID_STL=c++_shared",
                    "-DNOVA_ROOT_DIR=$novaRootDir",
                )
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    buildFeatures {
        compose = true
        aidl = false
        buildConfig = false
        shaders = false
    }

    externalNativeBuild {
        cmake {
            path = file("../../native/CMakeLists.txt")
            version = "3.31.6"
        }
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}

kotlin {
    jvmToolchain(21)
}

tasks.named("preBuild") {
    dependsOn(buildRustCore)
}

dependencies {
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation("androidx.compose.material:material-icons-extended")

    debugImplementation(libs.androidx.compose.ui.tooling)

    testImplementation(libs.junit)
}
