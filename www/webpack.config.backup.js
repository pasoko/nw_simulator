const path = require('path');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
    clean: true,
  },
  mode: 'development',
  devtool: 'source-map',
  plugins: [
    // Simply copy index.html without processing
    new CopyPlugin({
      patterns: [
        { from: 'index.html', to: 'index.html' },
        { from: 'pkg', to: 'pkg' },
        { from: 'debug.html', to: 'debug.html', noErrorOnMissing: true },
        { from: 'test.html', to: 'test.html', noErrorOnMissing: true }
      ],
    }),
  ],
  resolve: {
    extensions: ['.js', '.wasm'],
  },
  experiments: {
    asyncWebAssembly: true
  },
  devServer: {
    static: {
      directory: path.join(__dirname, 'dist'),
    },
    compress: true,
    port: 8080,
  },
};