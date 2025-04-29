package com.example.packfromjava;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Bundle;
import android.util.Log;
import android.view.View;
import androidx.activity.EdgeToEdge;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.graphics.Insets;
import androidx.core.view.ViewCompat;
import androidx.core.view.WindowInsetsCompat;
import java.io.IOException;
import java.io.OutputStream;

public class MainActivity extends AppCompatActivity {

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        EdgeToEdge.enable(this);
        setContentView(R.layout.activity_main);
        ViewCompat.setOnApplyWindowInsetsListener(
            findViewById(R.id.main),
            (v, insets) -> {
                Insets systemBars = insets.getInsets(
                    WindowInsetsCompat.Type.systemBars()
                );
                v.setPadding(
                    systemBars.left,
                    systemBars.top,
                    systemBars.right,
                    systemBars.bottom
                );
                return insets;
            }
        );
    }

    public void onApkClick(View v) {
        var samplePackage = createSamplePackage();
        var apk = samplePackage.compileApk();
        saveFileAs(
            "output.apk",
            "application/vnd.android.package-archive",
            apk
        );
    }

    public void onAabClick(View v) {
        var samplePackage = createSamplePackage();
        var aab = samplePackage.compileAab();
        saveFileAs("output.aab", "application/x-authorware-bin", aab);
    }

    private PackPackage createSamplePackage() {
        var samplePackage = new PackPackage();
        samplePackage.combinedPemString = StaticExampleData.COMBINED_PEM_STRING;

        samplePackage.androidManifest = StaticExampleData.ANDROID_MANIFEST;

        var watch_face_info = PackPackage.Resource.fromStringContents(
            "xml",
            "watch_face_info.xml",
            StaticExampleData.WATCH_FACE_INFO
        );
        samplePackage.resources.add(watch_face_info);

        var strings = PackPackage.Resource.fromStringContents(
            "values",
            "strings.xml",
            StaticExampleData.STRINGS
        );
        samplePackage.resources.add(strings);

        var watchface = PackPackage.Resource.fromStringContents(
            "raw",
            "watchface.xml",
            StaticExampleData.WATCH_FACE
        );
        samplePackage.resources.add(watchface);

        var preview = PackPackage.Resource.fromBase64Contents(
            "drawable",
            "preview.png",
            StaticExampleData.PREVIEW_PNG
        );
        samplePackage.resources.add(preview);

        return samplePackage;
    }

    /*

        The code below is just related to showing a "Save as..." dialog for the APK.
        It's not necessary/related to compiling APKs on-device.

     */

    private static final int SAVE_FILE_AS = 0;
    private byte[] fileToWrite;

    private void saveFileAs(String fileName, String mimeType, byte[] contents) {
        Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType(mimeType);
        intent.putExtra(Intent.EXTRA_TITLE, fileName);

        fileToWrite = contents;
        this.startActivityForResult(intent, SAVE_FILE_AS);
    }

    @Override
    public void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode == SAVE_FILE_AS && resultCode == Activity.RESULT_OK) {
            Uri uri = data.getData();

            try {
                OutputStream output = getApplicationContext()
                    .getContentResolver()
                    .openOutputStream(uri);

                output.write(fileToWrite);
                output.flush();
                output.close();
            } catch (IOException | NullPointerException e) {
                Log.e("MainActivity", "Failed to save file");
            }
        }
    }
}
