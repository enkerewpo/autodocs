# run command: cargo run -- run hvisor-book.yml
# every 1 minute

TIME_INTERVAL=60

while true; do
    echo "Running auto-translation at time: $(date)"
    cargo run -- run hvisor-book.yml
    echo "Auto-translation finished at time: $(date)"
    cp -r ./workspace/hvisor-book-translated/** ../hvisor-book-en
    cd ../hvisor-book-en
    git add .
    git commit -m "Auto-translation update at $(date)"
    git push
    echo "Auto-translation pushed to remote repository at time: $(date)"
    mdbook build
    cp -r ./book/** /www/wwwroot/hvisor-en.wheatfox.dev
    echo "Book built and deployed!"
    echo "Sleeping for $TIME_INTERVAL seconds"
    cd ../autodocs
    sleep $TIME_INTERVAL
done