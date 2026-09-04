const path = require('path');
const { getDefaultConfig, mergeConfig } = require('@react-native/metro-config');

// The library lives outside the app root (../qvp-react-native, symlinked into node_modules).
const lib = path.resolve(__dirname, '../qvp-react-native');
const config = {
  watchFolders: [lib],
  resolver: {
    nodeModulesPaths: [path.resolve(__dirname, 'node_modules')],
    extraNodeModules: {
      react: path.resolve(__dirname, 'node_modules/react'),
      'react-native': path.resolve(__dirname, 'node_modules/react-native'),
    },
  },
};

module.exports = mergeConfig(getDefaultConfig(__dirname), config);
