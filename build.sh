for i in i686-unknown-linux-gnu i686-pc-windows-gnu x86_64-pc-windows-gnu x86_64-unknown-linux-gnu
do 
    cargo build --release --target $i
done