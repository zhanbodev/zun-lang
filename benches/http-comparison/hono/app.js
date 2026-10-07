// Hono benchmark fixture — equivalent endpoints to zun and FastAPI.
// Using @hono/node-server because the sandbox / typical dev box has Node,
// not necessarily Bun. A Bun variant is trivial if you want an even faster
// comparison — replace the import with `import { serve } from 'bun'`.
import { Hono } from 'hono';
import { serve } from '@hono/node-server';

const app = new Hono();

app.get('/health', (c) => c.json({ status: 'ok' }));
app.get('/hello', (c) => c.json({ message: 'Hello, world!' }));
app.get('/greet/:name', (c) => {
  const name = c.req.param('name');
  return c.json({ message: `Hello, ${name}!` });
});

serve({ fetch: app.fetch, hostname: '127.0.0.1', port: 8080 });
