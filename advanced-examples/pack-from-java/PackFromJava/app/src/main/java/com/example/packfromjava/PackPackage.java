/*
 * Copyright 2025 Google LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.example.packfromjava;

import android.util.Log;
import java.util.ArrayList;
import java.util.Base64;
import java.util.List;

public class PackPackage {

    public static class Resource {

        public String subdirectory;
        public String name;
        public String contentsBase64;

        public static Resource fromBase64Contents(
            String subdirectory,
            String name,
            String contentsBase64
        ) {
            var resource = new Resource();
            resource.subdirectory = subdirectory;
            resource.name = name;
            resource.contentsBase64 = contentsBase64;
            return resource;
        }

        // Use this for binary assets like preview.png
        public static Resource fromByteArrayContents(
            String subdirectory,
            String name,
            byte[] contentsBytes
        ) {
            return Resource.fromBase64Contents(
                subdirectory,
                name,
                Base64.getEncoder().encodeToString(contentsBytes)
            );
        }

        // Use this for text files like strings.xml
        public static Resource fromStringContents(
            String subdirectory,
            String name,
            String contentsString
        ) {
            return Resource.fromByteArrayContents(
                subdirectory,
                name,
                contentsString.getBytes()
            );
        }
    }

    public String androidManifest;
    public List<Resource> resources = new ArrayList<>();
    public String combinedPemString;

    public byte[] compileApk() {
        return compilePackage(/* apk= */true);
    }

    public byte[] compileAab() {
        return compilePackage(/* apk= */false);
    }

    private byte[] compilePackage(boolean apk) {
        var resourceArray = new Resource[resources.size()];
        resources.toArray(resourceArray);
        var resultBase64 = nativeCompilePackage(
            androidManifest,
            resourceArray,
            combinedPemString,
            apk
        );
        return Base64.getDecoder().decode(resultBase64);
    }

    // The code here links in and provides the signature of the Rust library, "pack-java".
    private static native String nativeCompilePackage(
        String androidManifest,
        Resource[] resources,
        String combinedPemString,
        boolean apk
    );

    static {
        System.loadLibrary("pack_java");
    }
}
