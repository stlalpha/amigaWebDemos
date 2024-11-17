wasm-pack build --target web
rm -rf www/pkg  # Remove old pkg directory if it exists
cp -r pkg www/  # Copy new pkg directory
#cd www && basic-http-server .
