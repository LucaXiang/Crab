buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        classpath("com.android.tools.build:gradle:8.11.0")
        classpath("org.jetbrains.kotlin:kotlin-gradle-plugin:1.9.25")
    }
}

allprojects {
    repositories {
        google()
        mavenCentral()
    }
}

subprojects {
    afterEvaluate {
        if (extensions.findByName("android") != null) {
            val android = extensions.findByName("android") as com.android.build.gradle.BaseExtension
            android.compileSdkVersion(36)
            android.buildToolsVersion = "36.1.0"
        }
    }
}

tasks.register("clean").configure {
    delete("build")
}

