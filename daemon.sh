# run command: cargo run -- run hvisor-book.yml
# every 1 minute

TIME_INTERVAL=60

while true; do
    echo "Running auto-translation at time: $(date)"
    cargo run -- run hvisor-book.yml
    echo "Auto-translation finished at time: $(date)"
    cp -r ./workspace/hvisor-book-translated ../hvisor-book-en
    cd ../hvisor-book-en
    git add .
    git commit -m "Auto-translation update at $(date)"
    git push
    cd ../hvisor-book
    echo "Auto-translation pushed to remote repository at time: $(date)"
    echo "Sleeping for $TIME_INTERVAL seconds"
    sleep $TIME_INTERVAL
done