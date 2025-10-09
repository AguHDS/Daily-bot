use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use tokio::sync::RwLock as AsyncRwLock;

use crate::domain::entities::user_preferences::UserPreferences;
use crate::domain::repositories::user_preferences_repository::{
    RepositoryError, UserPreferencesRepository,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPreferencesData {
    users: HashMap<u64, UserPreferences>,
}

pub struct JsonUserPreferencesRepository {
    file_path: PathBuf,
    data: AsyncRwLock<HashMap<u64, UserPreferences>>,
}

impl JsonUserPreferencesRepository {
    pub fn new(file_path: impl Into<PathBuf>) -> Self {
        let file_path = file_path.into();
        println!(
            "💾 [DEBUG] JsonUserPreferencesRepository nuevo con ruta: {:?}",
            file_path
        );

        let data = Self::load_data(&file_path).unwrap_or_default();
        println!("💾 [DEBUG] Datos cargados: {} usuarios", data.len());

        Self {
            file_path,
            data: AsyncRwLock::new(data),
        }
    }

    fn load_data(file_path: &PathBuf) -> Result<HashMap<u64, UserPreferences>, RepositoryError> {
        println!("💾 [DEBUG] load_data llamado para: {:?}", file_path);

        if !file_path.exists() {
            println!("💾 [DEBUG] Archivo no existe, retornando HashMap vacío");
            return Ok(HashMap::new());
        }

        println!("💾 [DEBUG] Leyendo archivo existente");
        let content = fs::read_to_string(file_path).map_err(|e| {
            println!("❌ [DEBUG] Error leyendo archivo: {}", e);
            RepositoryError::StorageError(format!("Failed to read file: {}", e))
        })?;

        println!(
            "💾 [DEBUG] Parseando JSON, longitud: {} caracteres",
            content.len()
        );
        let data: UserPreferencesData = serde_json::from_str(&content).map_err(|e| {
            println!("❌ [DEBUG] Error parseando JSON: {}", e);
            RepositoryError::StorageError(format!("Failed to parse JSON: {}", e))
        })?;

        println!(
            "💾 [DEBUG] JSON parseado exitosamente, {} usuarios",
            data.users.len()
        );
        Ok(data.users)
    }

    async fn save_data(&self) -> Result<(), RepositoryError> {
        println!("💾 [DEBUG] save_data llamado");

        // Leer datos
        let data = {
            let lock = self.data.read().await;
            lock.clone() // Clonar para liberar el lock inmediatamente
        };

        println!("💾 [DEBUG] Datos a guardar: {} usuarios", data.len());

        let user_prefs_data = UserPreferencesData { users: data };

        // Serializar
        let json = serde_json::to_string_pretty(&user_prefs_data).map_err(|e| {
            println!("❌ [DEBUG] Error serializando JSON: {}", e);
            RepositoryError::StorageError(format!("Failed to serialize JSON: {}", e))
        })?;

        println!(
            "💾 [DEBUG] JSON serializado, longitud: {} caracteres",
            json.len()
        );

        // Crear directorio
        if let Some(parent) = self.file_path.parent() {
            println!("💾 [DEBUG] Creando directorio: {:?}", parent);
            fs::create_dir_all(parent).map_err(|e| {
                println!("❌ [DEBUG] Error creando directorio: {}", e);
                RepositoryError::StorageError(format!("Failed to create directory: {}", e))
            })?;
        }

        // Escribir archivo
        println!("💾 [DEBUG] Escribiendo archivo: {:?}", self.file_path);
        fs::write(&self.file_path, &json).map_err(|e| {
            println!("❌ [DEBUG] Error escribiendo archivo: {}", e);
            RepositoryError::StorageError(format!("Failed to write file: {}", e))
        })?;

        println!(
            "✅ [DEBUG] Archivo guardado exitosamente en: {:?}",
            self.file_path
        );
        Ok(())
    }
}

#[async_trait]
impl UserPreferencesRepository for JsonUserPreferencesRepository {
    async fn get(&self, user_id: u64) -> Result<Option<UserPreferences>, RepositoryError> {
        println!("💾 [DEBUG] get llamado para usuario: {}", user_id);
        let data = self.data.read().await;
        let result = data.get(&user_id).cloned();
        println!("💾 [DEBUG] get resultado: {:?}", result.is_some());
        Ok(result)
    }

    async fn save(&self, preferences: &UserPreferences) -> Result<(), RepositoryError> {
        println!(
            "💾 [DEBUG] save llamado para usuario: {}",
            preferences.user_id
        );
        println!("💾 [DEBUG] Preferencias: {:?}", preferences);

        let mut data = self.data.write().await;

        if !preferences.is_valid() {
            println!("❌ [DEBUG] Preferencias inválidas");
            return Err(RepositoryError::InvalidData(
                "Invalid user preferences".to_string(),
            ));
        }

        println!("💾 [DEBUG] Insertando usuario en memoria");
        data.insert(preferences.user_id, preferences.clone());
        println!(
            "💾 [DEBUG] Usuario insertado, total en memoria: {}",
            data.len()
        );

        // Liberar el lock antes de guardar
        drop(data);

        println!("💾 [DEBUG] Llamando a save_data");
        match self.save_data().await {
            Ok(()) => {
                println!("✅ [DEBUG] save completado exitosamente");
                Ok(())
            }
            Err(e) => {
                println!("❌ [DEBUG] Error en save_data: {:?}", e);
                Err(e)
            }
        }
    }

    async fn delete(&self, user_id: u64) -> Result<(), RepositoryError> {
        println!("💾 [DEBUG] delete llamado para usuario: {}", user_id);
        let mut data = self.data.write().await;

        if data.remove(&user_id).is_none() {
            println!("❌ [DEBUG] Usuario no encontrado para eliminar");
            return Err(RepositoryError::NotFound);
        }

        println!("💾 [DEBUG] Usuario eliminado, llamando a save_data");
        self.save_data().await
    }
}

impl std::fmt::Debug for JsonUserPreferencesRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JsonUserPreferencesRepository")
            .field("file_path", &self.file_path)
            .finish()
    }
}
