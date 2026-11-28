import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

interface Props {
  content: string;
}

export function MarkdownMessage({ content }: Props) {
  return (
    <div className="traveller-md text-sm leading-relaxed space-y-2 break-words">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => (
            <h1 className="text-base font-bold text-cyan-300 border-b border-slate-800 pb-1 mt-2">
              {children}
            </h1>
          ),
          h2: ({ children }) => (
            <h2 className="text-sm font-bold text-cyan-300 mt-2">{children}</h2>
          ),
          h3: ({ children }) => (
            <h3 className="text-xs font-bold uppercase tracking-wider text-amber-300 mt-2">
              {children}
            </h3>
          ),
          p: ({ children }) => <p className="leading-relaxed">{children}</p>,
          ul: ({ children }) => (
            <ul className="list-disc pl-5 space-y-1 text-slate-200">{children}</ul>
          ),
          ol: ({ children }) => (
            <ol className="list-decimal pl-5 space-y-1 text-slate-200">{children}</ol>
          ),
          li: ({ children }) => <li className="leading-relaxed">{children}</li>,
          strong: ({ children }) => (
            <strong className="font-semibold text-white">{children}</strong>
          ),
          em: ({ children }) => <em className="italic text-slate-300">{children}</em>,
          blockquote: ({ children }) => (
            <blockquote className="border-l-2 border-cyan-500/70 pl-3 py-0.5 text-slate-300 bg-slate-950/50 rounded-r">
              {children}
            </blockquote>
          ),
          code: ({ className, children }) => {
            const isBlock = Boolean(className);
            if (isBlock) {
              return (
                <code className="block bg-slate-950 border border-slate-800 rounded p-2.5 text-xs font-mono text-cyan-200 overflow-x-auto">
                  {children}
                </code>
              );
            }
            return (
              <code className="bg-slate-950/90 border border-slate-800 rounded px-1 py-0.5 text-xs font-mono text-amber-300">
                {children}
              </code>
            );
          },
          pre: ({ children }) => <pre className="my-1.5 overflow-x-auto">{children}</pre>,
          hr: () => <hr className="border-slate-800 my-2" />,
          table: ({ children }) => (
            <div className="overflow-x-auto my-2">
              <table className="w-full text-xs border-collapse border border-slate-800">
                {children}
              </table>
            </div>
          ),
          thead: ({ children }) => <thead className="bg-slate-950 text-cyan-300">{children}</thead>,
          th: ({ children }) => (
            <th className="border border-slate-800 px-2 py-1 text-left font-semibold">{children}</th>
          ),
          td: ({ children }) => (
            <td className="border border-slate-800 px-2 py-1 text-slate-200">{children}</td>
          ),
          a: ({ href, children }) => (
            <a
              href={href}
              target="_blank"
              rel="noreferrer"
              className="text-cyan-400 underline hover:text-cyan-300"
            >
              {children}
            </a>
          ),
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
