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
  experiments: {
    asyncWebAssembly: true
  },
  optimization: {
    minimize: true
  }
};