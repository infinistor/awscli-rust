//! 확장자 -> MIME 형식 표. .NET `AmazonS3Util.MimeTypeFromExtension`(AWSSDK.S3 4.x)의 내부 사전(`extensionToMime`)을
//! 리플렉션으로 꺼내 그대로 옮겼다. 확장자는 소문자 기준이며 맨 앞의 `.`을 포함한다.

/// (확장자, MIME 형식). 확장자 오름차순으로 정렬되어 있어 이진 탐색한다.
const TABLE: &[(&str, &str)] = &[
    (".ai", "application/postscript"),
    (".aif", "audio/x-aiff"),
    (".aifc", "audio/x-aiff"),
    (".aiff", "audio/x-aiff"),
    (".asc", "text/plain"),
    (".au", "audio/basic"),
    (".avi", "video/x-msvideo"),
    (".bcpio", "application/x-bcpio"),
    (".bin", "application/octet-stream"),
    (".c", "text/plain"),
    (".cc", "text/plain"),
    (".ccad", "application/clariscad"),
    (".cdf", "application/x-netcdf"),
    (".class", "application/octet-stream"),
    (".cpio", "application/x-cpio"),
    (".cpp", "text/plain"),
    (".cpt", "application/mac-compactpro"),
    (".cs", "text/plain"),
    (".csh", "application/x-csh"),
    (".css", "text/css"),
    (".csv", "text/csv"),
    (".dcr", "application/x-director"),
    (".dir", "application/x-director"),
    (".dms", "application/octet-stream"),
    (".doc", "application/msword"),
    (
        ".docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ),
    (".dot", "application/msword"),
    (".drw", "application/drafting"),
    (".dvi", "application/x-dvi"),
    (".dwg", "application/acad"),
    (".dxf", "application/dxf"),
    (".dxr", "application/x-director"),
    (".eps", "application/postscript"),
    (".etx", "text/x-setext"),
    (".exe", "application/octet-stream"),
    (".ez", "application/andrew-inset"),
    (".f", "text/plain"),
    (".f90", "text/plain"),
    (".fli", "video/x-fli"),
    (".gif", "image/gif"),
    (".gtar", "application/x-gtar"),
    (".gz", "application/x-gzip"),
    (".h", "text/plain"),
    (".hdf", "application/x-hdf"),
    (".hh", "text/plain"),
    (".hqx", "application/mac-binhex40"),
    (".htm", "text/html"),
    (".html", "text/html"),
    (".ice", "x-conference/x-cooltalk"),
    (".ief", "image/ief"),
    (".iges", "model/iges"),
    (".igs", "model/iges"),
    (".ips", "application/x-ipscript"),
    (".ipx", "application/x-ipix"),
    (".jpe", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".jpg", "image/jpeg"),
    (".js", "application/x-javascript"),
    (".json", "application/json"),
    (".kar", "audio/midi"),
    (".latex", "application/x-latex"),
    (".lha", "application/octet-stream"),
    (".lsp", "application/x-lisp"),
    (".lzh", "application/octet-stream"),
    (".m", "text/plain"),
    (".m3u8", "application/x-mpegURL"),
    (".m4v", "video/x-m4v"),
    (".man", "application/x-troff-man"),
    (".me", "application/x-troff-me"),
    (".mesh", "model/mesh"),
    (".mid", "audio/midi"),
    (".midi", "audio/midi"),
    (".mime", "www/mime"),
    (".mov", "video/quicktime"),
    (".movie", "video/x-sgi-movie"),
    (".mp2", "audio/mpeg"),
    (".mp3", "audio/mpeg"),
    (".mp4", "video/mp4"),
    (".mpe", "video/mpeg"),
    (".mpeg", "video/mpeg"),
    (".mpg", "video/mpeg"),
    (".mpga", "audio/mpeg"),
    (".ms", "application/x-troff-ms"),
    (".msh", "model/mesh"),
    (".msi", "application/x-ole-storage"),
    (".nc", "application/x-netcdf"),
    (".oda", "application/oda"),
    (".ogv", "video/ogv"),
    (".pbm", "image/x-portable-bitmap"),
    (".pdb", "chemical/x-pdb"),
    (".pdf", "application/pdf"),
    (".pgm", "image/x-portable-graymap"),
    (".pgn", "application/x-chess-pgn"),
    (".png", "image/png"),
    (".pnm", "image/x-portable-anymap"),
    (".pot", "application/mspowerpoint"),
    (".ppm", "image/x-portable-pixmap"),
    (".pps", "application/mspowerpoint"),
    (".ppt", "application/mspowerpoint"),
    (
        ".pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ),
    (".ppz", "application/mspowerpoint"),
    (".pre", "application/x-freelance"),
    (".prt", "application/pro_eng"),
    (".ps", "application/postscript"),
    (".qt", "video/quicktime"),
    (".ra", "audio/x-realaudio"),
    (".ram", "audio/x-pn-realaudio"),
    (".ras", "image/cmu-raster"),
    (".rgb", "image/x-rgb"),
    (".rm", "audio/x-pn-realaudio"),
    (".roff", "application/x-troff"),
    (".rpm", "audio/x-pn-realaudio-plugin"),
    (".rtf", "text/rtf"),
    (".rtx", "text/richtext"),
    (".scm", "application/x-lotusscreencam"),
    (".set", "application/set"),
    (".sgm", "text/sgml"),
    (".sgml", "text/sgml"),
    (".sh", "application/x-sh"),
    (".shar", "application/x-shar"),
    (".silo", "model/mesh"),
    (".sit", "application/x-stuffit"),
    (".skd", "application/x-koan"),
    (".skm", "application/x-koan"),
    (".skp", "application/x-koan"),
    (".skt", "application/x-koan"),
    (".smi", "application/smil"),
    (".smil", "application/smil"),
    (".snd", "audio/basic"),
    (".sol", "application/solids"),
    (".spl", "application/x-futuresplash"),
    (".src", "application/x-wais-source"),
    (".step", "application/STEP"),
    (".stl", "application/SLA"),
    (".stp", "application/STEP"),
    (".sv4cpio", "application/x-sv4cpio"),
    (".sv4crc", "application/x-sv4crc"),
    (".svg", "image/svg+xml"),
    (".swf", "application/x-shockwave-flash"),
    (".t", "application/x-troff"),
    (".tar", "application/x-tar"),
    (".tcl", "application/x-tcl"),
    (".tex", "application/x-tex"),
    (".tif", "image/tiff"),
    (".tiff", "image/tiff"),
    (".tr", "application/x-troff"),
    (".ts", "video/MP2T"),
    (".tsi", "audio/TSP-audio"),
    (".tsp", "application/dsptype"),
    (".tsv", "text/tab-separated-values"),
    (".txt", "text/plain"),
    (".unv", "application/i-deas"),
    (".ustar", "application/x-ustar"),
    (".vcd", "application/x-cdlink"),
    (".vda", "application/vda"),
    (".vrml", "model/vrml"),
    (".wav", "audio/x-wav"),
    (".webm", "video/webm"),
    (".wmv", "video/x-ms-wmv"),
    (".wrl", "model/vrml"),
    (".xap", "application/x-silverlight-app"),
    (".xbm", "image/x-xbitmap"),
    (".xlc", "application/vnd.ms-excel"),
    (".xll", "application/vnd.ms-excel"),
    (".xlm", "application/vnd.ms-excel"),
    (".xls", "application/vnd.ms-excel"),
    (
        ".xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ),
    (".xlw", "application/vnd.ms-excel"),
    (".xml", "text/xml"),
    (".xpm", "image/x-xpixmap"),
    (".xwd", "image/x-xwindowdump"),
    (".xyz", "chemical/x-pdb"),
    (".zip", "application/zip"),
];

/// 원본 `AmazonS3Util.MimeTypeFromExtension`: 확장자(`.txt`)로 MIME 형식을 찾고, 없으면 `application/octet-stream`.
pub fn mime_type_from_extension(extension: &str) -> &'static str {
    let lower = extension.to_ascii_lowercase();
    TABLE
        .binary_search_by(|(ext, _)| (*ext).cmp(lower.as_str()))
        .map(|i| TABLE[i].1)
        .unwrap_or("application/octet-stream")
}

/// .NET `Path.GetExtension`: 마지막 경로 구분자 뒤에서 마지막 `.`부터 끝까지(`.` 포함). `.`이 없거나 맨 끝이면 빈 문자열.
pub fn path_extension(path: &str) -> &str {
    let name_start = path
        .rfind(['/', std::path::MAIN_SEPARATOR])
        .map_or(0, |i| i + 1);
    let name = &path[name_start..];
    match name.rfind('.') {
        Some(i) if i + 1 < name.len() => &name[i..],
        _ => "",
    }
}

/// `PutObject`의 `Content-Type` 기본값: 확장자가 없으면 `text/plain`, 있으면 표에서 찾는다(.NET 동작을 캡처해 확인).
pub fn put_object_content_type(path: &str) -> &'static str {
    match path_extension(path) {
        "" => "text/plain",
        ext => mime_type_from_extension(ext),
    }
}

/// `CreateMultipartUpload`의 `Content-Type` 기본값: 확장자가 없으면 헤더를 보내지 않는다.
pub fn initiate_content_type(path: &str) -> Option<&'static str> {
    match path_extension(path) {
        "" => None,
        ext => Some(mime_type_from_extension(ext)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted() {
        assert!(TABLE.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn path_extension_matches_dotnet() {
        assert_eq!(path_extension("dir/key.txt"), ".txt");
        assert_eq!(path_extension("noext"), "");
        assert_eq!(path_extension("dir.d/noext"), "");
        assert_eq!(path_extension("a."), "");
        assert_eq!(path_extension(".hidden"), ".hidden");
        assert_eq!(path_extension("a.tar.gz"), ".gz");
        assert_eq!(path_extension("dir/"), "");
    }

    #[test]
    fn known_types() {
        assert_eq!(mime_type_from_extension(".xyz"), "chemical/x-pdb");
        assert_eq!(mime_type_from_extension(".TXT"), "text/plain");
        assert_eq!(
            mime_type_from_extension(".unknown"),
            "application/octet-stream"
        );
        assert_eq!(put_object_content_type("a"), "text/plain");
        assert_eq!(initiate_content_type("a"), None);
        assert_eq!(
            initiate_content_type(".hidden"),
            Some("application/octet-stream")
        );
    }
}
