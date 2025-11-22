#### Cambiador de Nombres

- **Targets y Configuración Aleatoria:**  
  `src/features/server_specific/config/nickname_targets.rs` -> users IDs, pool de nombres y configuración de probabilidad, enable/disable nickname changer

- **Mensaje de Notificación:**  
  `src/features/server_specific/services/nickname_changer.rs` -> mensaje que envía el bot al cambiar nombre

- **Scheduler Aleatorio:**  
  `src/features/server_specific/scheduler/nickname_scheduler.rs` -> aplica probabilidad de cambio cada 5 minutos

#### Configuración del Server

- **Server de Bromas:**  
  `src/features/server_specific/config/server_config.rs` -> server ID y channel ID para features de broma

- **Inicialización Features:**  
  `src/infrastructure/discord_bot/bot.rs` -> cambiar my_server_id para activar features en server específico