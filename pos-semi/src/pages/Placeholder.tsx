export default function Placeholder({ title, note }: { title: string; note: string }) {
  return (
    <div className="pos-placeholder">
      <h2>{title}</h2>
      <p>{note}</p>
    </div>
  );
}
