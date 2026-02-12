# SvS Preparation Week - Appointment Scheduler

A Rust-based appointment scheduling system for SvS Preparation Week with a web interface.

## Features

- **Form-Based System**: Create custom forms with configurable alliances, time slots, and requirements
- **Multi-Language Support**: Form submission page supports English, Korean, Chinese, and Japanese
- **Smart Scheduling**: Automatic scheduling with priority-based slot assignment from form submissions
- **Manual Schedule Editing**: Click-to-edit schedule slots to manually assign or change players
- **Slot Stealing**: Advanced algorithm that can move players up to 5 levels deep to optimize assignments
- **Predetermined Slots**: Pre-assign specific time slots to players before schedule generation. Bidirectional link: assigning research slot 1 automatically gives construction last slot, and assigning construction last slot automatically gives research slot 1.
- **Append Mode**: Option to append to an existing schedule instead of replacing it—keeps current assignments and fills only empty slots with new form submissions.
- **Day-Specific Logic**: 
  - Construction Day: Prioritizes slot 49 for players who want research and have slot 1 available
  - Research Day: Automatically locks slot 1 for the player in Construction Day's slot 49
  - Troops Training Day: Standard priority-based scheduling
- **Web Interface**: 
  - Dashboard with tabs for Schedule, Statistics, Create Form, Current Form, CSV Operations, and Generate Schedule
  - Session-based authentication
  - Statistics page showing alliance counts and time slot popularity per day
  - Schedule display with manual editing capabilities
  - Form submission data table
- **CSV Support**: Optional CSV upload for legacy data or bulk imports
- **Modular Architecture**: Clean separation of concerns with dedicated modules for parsing, scheduling, and display

## Usage

### CLI Mode

```bash
cargo run
```

This will:
1. Process the CSV file (looks for `data/testData2.csv` or falls back to a hardcoded path)
2. Generate schedules for all three days
3. Print schedules to the terminal
4. Write schedule files: `schedule_construction.txt`, `schedule_research.txt`, `schedule_troops.txt`

### Web Server Mode

```bash
# Set admin password (optional, defaults to "admin123")
$env:ADMIN_PASSWORD="your-secure-password"

# Start web server on port 8080 (default)
cargo run web

# Or specify a custom port
cargo run web 3000
```

Then access:
- Home: http://localhost:8080
- Create Account: http://localhost:8080/create-account
- Dashboard: http://localhost:8080/dashboard/{account_name} (after login)
- Public Form: http://localhost:8080/form/{form_code}
- Legacy Admin Panel: http://localhost:8080/{account_name}/{server}/admin (for CSV upload)

## Main Workflow

1. **Create Account**: Register with account name, server number, and password
2. **Login**: Access your dashboard
3. **Create Form**: Set up a form with alliances, time slots, and requirements
4. **Share Form**: Share the form link with players
5. **Generate Schedule**: Generate optimized schedules from form submissions
6. **Edit Schedule**: Manually edit any schedule slot as needed
7. **View Data**: Check all form submissions in the Current Form tab

## CSV Upload (Optional)

CSV upload is available as an alternative method:
1. Navigate to Dashboard → CSV Operations tab
2. Upload a CSV file
3. The system will process and generate schedules

## Statistics

Shows:
- **Alliance Request Counts**: How many requests from each alliance for each day type
- **Time Slot Popularity**: Separate statistics for Construction, Research, and Troops days
- **Per-Day Breakdown**: Each day type has its own time slot popularity display

**Note**: No player names are displayed on the statistics page for privacy.

## Schedule Display

View and edit the complete schedules for:
- Construction Day
- Research Day
- Troops Training Day

Each schedule shows all time slots with assigned players or [EMPTY] markers. Click any slot to manually edit the player assignment.

## Deployment to name.com

To deploy to name.com hosting:

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Upload the binary and required files:
   - `target/release/prep-appointments.exe` (or binary for your server OS)
   - `templates/` directory
   - `static/` directory

3. Set up data directory structure:
   ```bash
   mkdir -p data/current_forms data/old_forms data/schedules data/statistics
   ```

4. Run the server:
   ```bash
   ./prep-appointments web 80
   ```

5. Configure your domain to point to the server and set up port forwarding if needed.

## Project Structure

```
prep-appointments/
├── src/
│   ├── main.rs           # Main entry point (CLI and web server launcher)
│   ├── parser.rs         # CSV parsing and AppointmentEntry struct
│   ├── display.rs        # Terminal output and file writing functions
│   ├── web.rs            # Web server and API endpoints
│   └── schedule/         # Scheduling algorithm modules
│       ├── mod.rs        # Module declarations and public exports
│       ├── types.rs      # Data structures (ScheduledAppointment, DaySchedule, Move)
│       ├── slot_utils.rs # Slot conversion and ranking utilities
│       ├── move_chain.rs # Slot reassignment chain logic
│       ├── generic.rs    # Generic scheduling functions
│       ├── construction.rs # Construction day scheduler
│       ├── research.rs   # Research day scheduler
│       └── troops.rs     # Troops training day scheduler
├── templates/            # HTML templates
│   ├── index.html
│   ├── admin.html
│   ├── stats.html
│   └── schedules.html
├── static/               # Static assets
│   └── style.css
└── Cargo.toml
```

### Module Overview

- **`parser.rs`**: Handles CSV file parsing, time slot conversion, and data validation. Contains the `AppointmentEntry` struct that represents each player's appointment preferences.
- **`schedule/`**: Contains the scheduling algorithms, organized by function:
  - **`types.rs`**: Core data structures used throughout the scheduling system
  - **`slot_utils.rs`**: Utility functions for converting between time strings and slot numbers, and calculating slot popularity
  - **`move_chain.rs`**: Implements the slot "stealing" mechanism that can rearrange players up to 5 levels deep
  - **`generic.rs`**: Generic scheduling functions used by all day types
  - **`construction.rs`**: Specialized logic for Construction Day (handles slot 49 priority)
  - **`research.rs`**: Specialized logic for Research Day (handles locked slot 1 from Construction Day)
  - **`troops.rs`**: Simple wrapper for Troops Training Day scheduling
- **`display.rs`**: Handles all output formatting, including terminal display and file writing
- **`web.rs`**: Web server implementation using Actix-web, handles API endpoints and serves HTML pages

## Security Note

**Important**: The system uses session-based authentication. Each account has its own password. Make sure to use strong passwords for production deployments.

