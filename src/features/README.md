**FEAURES SECTION IS ONLY MEANT TO BE USED IN MY PERSONAL SERVER, IT'S LOGIC IS USELESS FOR THE BOT'S PURPOSE**

### Guide

##### Cambiador de Nombres y Kick de usuarios

- **Targets para cambiar nombre:**  
  `src\features\server_specific\data\nickname_targets.json` -> % de probabilidad de cambio para cada usuario objetivo

- **Targets para kicks**:
  `src\features\server_specific\data\kick_targets.json` -> % de probabilidad de kick para cada usuario objetivo

##### Configuración del Server

- **Server de Bromas:**  
  `src/features/server_specific/config/server_config.rs` -> server ID y channel ID para specific features

- **Inicialización Features:**  
  `src/infrastructure/discord_bot/bot.rs` -> cambiar my_server_id para activar features en server específico