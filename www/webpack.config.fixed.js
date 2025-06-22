const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyPlugin = require('copy-webpack-plugin');

module.exports = {
  entry: './index.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'bundle.js',
    clean: true,
  },
  mode: 'production',
  devtool: false,
  plugins: [
    new HtmlWebpackPlugin({
      template: './index.html',
      inject: 'body',
      minify: {
        collapseWhitespace: false,
        removeComments: false,
        removeRedundantAttributes: false,
        removeScriptTypeAttributes: false,
        removeStyleLinkTypeAttributes: false,
        useShortDoctype: false
      }
    }),
    new CopyPlugin({
      patterns: [
        { from: 'pkg', to: 'pkg' },
        { from: 'simple-test.html', to: 'simple-test.html', noErrorOnMissing: true }
      ],
    }),
  ],
  experiments: {
    asyncWebAssembly: true
  },
  optimization: {
    minimize: true
  }
};