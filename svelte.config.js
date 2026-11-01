import adapter from '@sveltejs/adapter-static';

const config = {
  kit: {
    alias: {
      $src: 'src',
    },
    adapter: adapter({
      fallback: 'index.html',
    }),
  },
};

export default config;
