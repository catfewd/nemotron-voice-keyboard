use anyhow::anyhow;
use jni::objects::JObject;
use jni::JNIEnv;
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

pub struct MappedNemotron {
    pub encoder: Arc<Mmap>,
    pub decoder: Arc<Mmap>,
    pub tokenizer: Vec<u8>,
}

pub fn get_mapped_assets(env: &mut JNIEnv, context: &JObject) -> anyhow::Result<MappedNemotron> {
    // 1. Get APK path
    let application_info_obj = env.call_method(context, "getApplicationInfo", "()Landroid/content/pm/ApplicationInfo;", &[])?.l()?;
    let source_dir_j = env.get_field(&application_info_obj, "sourceDir", "Ljava/lang/String;")?.l()?;
    let apk_path: String = env.get_string(&source_dir_j.into())?.into();

    // 2. Map APK
    let file = File::open(&apk_path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let mmap_arc = Arc::new(mmap);

    // 3. Find offsets
    let zip_file = File::open(&apk_path)?;
    let mut zip = zip::ZipArchive::new(zip_file)?;
    
    // Check compression for encoder
    {
        let encoder_info = zip.by_name("assets/nemotron-model/encoder.onnx")?;
        if encoder_info.compression() != zip::CompressionMethod::Stored {
            return Err(anyhow!("Model must be stored uncompressed in APK! Check build.sh."));
        }
    }

    // Read tokenizer
    let tokenizer = {
        let mut tokenizer_info = zip.by_name("assets/nemotron-model/tokenizer.model")?;
        let mut buf = Vec::with_capacity(tokenizer_info.size() as usize);
        tokenizer_info.read_to_end(&mut buf)?;
        buf
    };

    Ok(MappedNemotron {
        encoder: mmap_arc.clone(),
        decoder: mmap_arc.clone(),
        tokenizer,
    })
}

pub fn get_asset_slice<'a>(mmap: &'a Mmap, asset_path: &str, apk_path: &str) -> anyhow::Result<&'a [u8]> {
    let zip_file = File::open(apk_path)?;
    let mut zip = zip::ZipArchive::new(zip_file)?;
    let file_info = zip.by_name(asset_path)?;
    let start = file_info.data_start() as usize;
    let end = start + file_info.size() as usize;
    Ok(&mmap[start..end])
}
