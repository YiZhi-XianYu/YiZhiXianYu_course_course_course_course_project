const http = require('http');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const root = path.resolve(__dirname, '..');
const htmlDir = __dirname;
const port = Number(process.env.PORT || 8787);
const jobs = new Map();

function sendJson(res, status, body) {
  const data = Buffer.from(JSON.stringify(body));
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'content-length': data.length,
  });
  res.end(data);
}

function readJson(req) {
  return new Promise((resolve, reject) => {
    let body = '';
    req.setEncoding('utf8');
    req.on('data', chunk => {
      body += chunk;
      if (body.length > 1024 * 1024) {
        reject(new Error('请求体过大'));
        req.destroy();
      }
    });
    req.on('end', () => {
      try {
        resolve(body ? JSON.parse(body) : {});
      } catch (err) {
        reject(err);
      }
    });
    req.on('error', reject);
  });
}

function resolveUserPath(value) {
  if (!value || typeof value !== 'string') return null;
  const trimmed = value.trim();
  if (!trimmed) return null;
  return path.resolve(root, trimmed);
}

function defaultOutput(input) {
  const parsed = path.parse(input || 'output.mp4');
  const stamp = new Date().toISOString().replace(/[-:.TZ]/g, '').slice(0, 14);
  return path.join(root, `${parsed.name}_web_${stamp}.mp4`);
}

function executablePath() {
  return path.join(root, 'target', 'release', 'subtitle-burner.exe');
}

function push(job, line, kind = 'log') {
  const event = { kind, line, time: new Date().toISOString() };
  job.events.push(event);
  for (const client of job.clients) {
    client.write(`data: ${JSON.stringify(event)}\n\n`);
  }
}

function finish(job, ok, payload) {
  job.done = true;
  job.ok = ok;
  job.result = payload;
  const event = { kind: ok ? 'done' : 'failed', ...payload, time: new Date().toISOString() };
  job.events.push(event);
  for (const client of job.clients) {
    client.write(`data: ${JSON.stringify(event)}\n\n`);
    client.end();
  }
  job.clients.clear();
}

function startJob(params) {
  const input = resolveUserPath(params.input);
  if (!input || !fs.existsSync(input)) {
    throw new Error('输入视频不存在，请检查路径。');
  }

  const request = String(params.request || '').trim();
  if (!request) {
    throw new Error('请输入处理需求。');
  }

  const output = params.output ? resolveUserPath(params.output) : defaultOutput(input);
  const outputDir = path.dirname(output);
  if (!fs.existsSync(outputDir)) {
    throw new Error(`输出目录不存在：${outputDir}`);
  }

  const sticker = params.sticker ? resolveUserPath(params.sticker) : path.join(root, 'image', 'image.png');
  const exe = executablePath();
  if (!fs.existsSync(exe)) {
    throw new Error('未找到 release 可执行文件，请先运行 cargo build --release。');
  }

  const id = `${Date.now()}-${Math.random().toString(16).slice(2)}`;
  const job = {
    id,
    input,
    output,
    request,
    events: [],
    clients: new Set(),
    done: false,
    ok: false,
    result: null,
  };
  jobs.set(id, job);

  const args = [
    'assistant',
    '--ask',
    request,
    '--input',
    input,
    '--output',
    output,
    '--keep-srt',
    '--verbose',
  ];
  if (sticker && fs.existsSync(sticker)) {
    args.push('--sticker', sticker);
  }

  push(job, `[server] ${exe} ${args.map(a => a.includes(' ') ? `"${a}"` : a).join(' ')}`, 'system');
  const child = spawn(exe, args, { cwd: root, windowsHide: true });

  child.stdout.setEncoding('utf8');
  child.stderr.setEncoding('utf8');
  child.stdout.on('data', data => {
    for (const line of data.split(/\r?\n/).filter(Boolean)) {
      push(job, line, 'log');
    }
  });
  child.stderr.on('data', data => {
    for (const line of data.split(/\r?\n/).filter(Boolean)) {
      push(job, line, 'stderr');
    }
  });
  child.on('error', err => finish(job, false, { message: err.message }));
  child.on('close', code => {
    if (code === 0 && fs.existsSync(output)) {
      finish(job, true, {
        message: '处理完成，已生成视频。',
        output,
        videoUrl: `/video?path=${encodeURIComponent(output)}&t=${Date.now()}`,
      });
      return;
    }

    if (code === 0) {
      finish(job, false, { message: '命令执行完成，但没有找到输出视频。' });
      return;
    }

    const lastLog = [...job.events].reverse().find(e => e.line);
    finish(job, false, {
      message: lastLog?.line || `处理失败，退出码 ${code}`,
      code,
    });
  });

  return job;
}

function serveFile(res, filePath, type) {
  fs.readFile(filePath, (err, data) => {
    if (err) {
      res.writeHead(404);
      res.end('not found');
      return;
    }
    res.writeHead(200, { 'content-type': type });
    res.end(data);
  });
}

function serveVideo(req, res, url) {
  const filePath = resolveUserPath(url.searchParams.get('path'));
  if (!filePath || !fs.existsSync(filePath)) {
    res.writeHead(404);
    res.end('video not found');
    return;
  }

  const stat = fs.statSync(filePath);
  const range = req.headers.range;
  if (!range) {
    res.writeHead(200, {
      'content-type': 'video/mp4',
      'content-length': stat.size,
      'accept-ranges': 'bytes',
    });
    fs.createReadStream(filePath).pipe(res);
    return;
  }

  const [startText, endText] = range.replace(/bytes=/, '').split('-');
  const start = Number(startText);
  const end = endText ? Number(endText) : stat.size - 1;
  if (!Number.isFinite(start) || !Number.isFinite(end) || start < 0 || end >= stat.size || start > end) {
    res.writeHead(416);
    res.end('invalid range');
    return;
  }

  res.writeHead(206, {
    'content-type': 'video/mp4',
    'content-length': end - start + 1,
    'content-range': `bytes ${start}-${end}/${stat.size}`,
    'accept-ranges': 'bytes',
  });
  fs.createReadStream(filePath, { start, end }).pipe(res);
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://127.0.0.1:${port}`);
  try {
    if (req.method === 'GET' && url.pathname === '/') {
      return serveFile(res, path.join(htmlDir, 'index.html'), 'text/html; charset=utf-8');
    }

    if (req.method === 'GET' && url.pathname === '/video') {
      return serveVideo(req, res, url);
    }

    if (req.method === 'POST' && url.pathname === '/api/run') {
      const body = await readJson(req);
      const job = startJob(body);
      return sendJson(res, 200, { jobId: job.id, output: job.output });
    }

    if (req.method === 'GET' && url.pathname.startsWith('/api/events/')) {
      const id = decodeURIComponent(url.pathname.split('/').pop());
      const job = jobs.get(id);
      if (!job) {
        res.writeHead(404);
        return res.end('job not found');
      }

      res.writeHead(200, {
        'content-type': 'text/event-stream; charset=utf-8',
        'cache-control': 'no-cache',
        connection: 'keep-alive',
      });

      for (const event of job.events) {
        res.write(`data: ${JSON.stringify(event)}\n\n`);
      }

      if (job.done) {
        res.end();
      } else {
        job.clients.add(res);
        req.on('close', () => job.clients.delete(res));
      }
      return;
    }

    res.writeHead(404);
    res.end('not found');
  } catch (err) {
    sendJson(res, 400, { error: err.message });
  }
});

server.listen(port, '127.0.0.1', () => {
  console.log(`Isekai Camera web server: http://127.0.0.1:${port}`);
});
