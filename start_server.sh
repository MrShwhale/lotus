set -o pipefail

LOGS_DIR="logs"
SCRAPER_LOGS="$LOGS_DIR/scraper"
SERVER_LOGS="$LOGS_DIR/server"
OUTPUT_BACKUP="output_backup"

# Ensure the log folders exist
if [ ! -d SCRAPER_LOGS ]; then
  echo "Scraper log directory not found."
  return 1
fi

if [ ! -d SERVER_LOGS ]; then
  echo "Server log directory not found."
  return 1
fi

# Keep a backup of the previous scrape
LOG_NAME=$(date +"%Y-%m-%d+%T")
$CURRENT_BACKUP="$OUTPUT_BACKUP/$LOG_NAME"
mkdir $CURRENT_BACKUP
mv "output/* $CURRENT_BACKUP"

# Start a scrape, outputting to console and to a log file
./lotus_scrape 2>&1 | tee "$SCRAPER_LOGS/$LOG_NAME"

# If the above fails, then something has gone wrong 
# and it should be looked at by an admin!
# Keep the current server running until they do.
if [ $? -eq 1 ]; then
    return;
fi

# Close the server, if open
pkill lotus_web

# Start the server, outputting to console and to a log file
LOG_NAME=$(date +"%Y-%m-%d+%T")
./lotus_web 2>&1 | tee "$SERVER_LOGS/$LOG_NAME"

# If the above fails, then something has gone wrong 
# and it should be looked at by an admin!
# Since the server is the part that's not working, 
# it obviously shouldn't be kept running.
if [ $? -eq 1 ]; then
    return;
fi
