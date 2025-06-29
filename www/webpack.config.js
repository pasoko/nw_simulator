const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
    clean: true,
    publicPath: '/',
  },
  mode: 'production',
  devtool: false,
  plugins: [
    new HtmlWebpackPlugin({
      template: './index.html',
      inject: 'body',
      minify: false
    }),
    new CopyPlugin({
      patterns: [
        { from: 'pkg', to: 'pkg' }
      ],
    }),
  ],
  // Disable webpack's WASM experiments to avoid hash issues
  experiments: {
    asyncWebAssembly: false
  },
  optimization: {
    minimize: true
  },
  performance: {
    hints: false, // Disable performance warnings
    maxAssetSize: 512000, // Increase limit to 512KB for WASM files
    maxEntrypointSize: 512000
  }
};