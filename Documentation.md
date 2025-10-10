### Scheduler

**File:** `src/application/scheduler/scheduler_tokio.rs`  
**Usage:** Called in `handlers.rs` inside the `ready` event of the `CommandHandler`

##### Description
The scheduler is an **asynchronous loop** that:

- Periodically checks the task repository
- Finds tasks whose **scheduled time has passed** and are **not yet completed**
- Prints a reminder message in the console for each task
- Marks tasks as completed to avoid repeated reminders

##### Purpose
- Automates the execution of scheduled tasks
- Simulates **automatic reminders** for users
- Can be extended in the future to **send messages directly in Discord** instead of just printing to the console

### Geo-Mapping service

**File:** `src\application\services\geo_mapping_service.rs`
Usage: Used by TimezoneService to resolve geographic queries into valid timezone identifiers

### Description
The Geo-Mapping Service provides a lightweight, in-memory geographic lookup system that maps countries, U.S. states, and Canadian provinces to their corresponding IANA timezone identifiers.
Exposes several lookup functions and a unified search method that:

- Converts geographic names (country, state, province) into timezone strings
- Handles case-insensitive input
- Searches across multiple predefined mappings
- Acts as a fallback mechanism for more precise or fuzzy timezone searches performed by the TimezoneManager
- All UTC timezones here: `src\infrastructure\data\timezones.json`

### Purpose
Provides fast, static geographic-to-timezone resolution without requiring a database
Simplifies timezone inference for user input such as “Brazil” or “California”
Enables the TimezoneService to perform combined searches using both static mappings and fuzzy timezone data
Serves as a foundation for a future database-backed GeoMappingRepository, allowing migration to MySQL or other persistent storage solutions

