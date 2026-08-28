use locus_engine::AstGuard;

#[test]
fn test_adversarial_suite_all_caught() {
    let cases = vec![
        // 1. Hook مموه داخل دالة فرعية مغلفة بشرط تفرع ثلاثي
        (
            r#"
            export function UserProfile({ isMember }: { isMember: boolean }) {
                const getHandler = isMember ? () => { const [val, setVal] = useState(0); return val; } : () => null;
                return <div>{getHandler()}</div>;
            }
            "#,
            "React Rules of Hooks: Hook داخل تفرع شرطي",
        ),
        // 2. ReDoS خبيث بتكرار متداخل غير محدود
        (
            r#"
            const emailRegex = /^(a+)+$/;
            export function validate(input: string) { return emailRegex.test(input); }
            "#,
            "ReDoS: تراجع كارثي في التعبيرات النمطية",
        ),
        // 3. قفل Mutex متزامن محتجز عبر نقطة await داخل ذراع match
        (
            r#"
            use std::sync::Mutex;
            pub async fn process_data(lock: &Mutex<State>) {
                match lock.lock() {
                    Ok(guard) => {
                        let _val = guard.count;
                        fetch_remote_data().await;
                    }
                    Err(_) => {}
                }
            }
            "#,
            "Async Mutex: احتجاز قفل متزامن عبر .await",
        ),
        // 4. تسريب مفتاح بيئة سري داخل مكون "use client"
        (
            r#"
            "use client";
            export function PaymentForm() {
                const secret = process.env.STRIPE_SECRET_KEY;
                return <button onClick={() => submit(secret)}>Pay</button>;
            }
            "#,
            "Secret Leak: تسريب متغيرات بيئة الخادم في العميل",
        ),
        // 5. حقن HTML خطير عبر dangerouslySetInnerHTML بدون تطهير
        (
            r#"
            export function RawViewer({ userHtml }: { userHtml: string }) {
                return <div dangerouslySetInnerHTML={{ __html: userHtml }} />;
            }
            "#,
            "XSS Injection: حقن HTML غير آمن ومباشر",
        ),
        // 6. قسمة حسابية غير محمية على متغير مدخل
        (
            r#"
            pub fn calculate_ratio(total: f64, count: f64) -> f64 {
                total / count
            }
            "#,
            "Division by Zero: قسمة غير مؤمنة على صفر",
        ),
        // 7. وصول عميق لخاصية كائن (5 مستويات) دون استخدام ?.
        (
            r#"
            export function getAuthorAvatar(response: any) {
                return response.data.article.meta.author.avatar.url;
            }
            "#,
            "Null Dereference: وصول عميق دون Optional Chaining",
        ),
        // 8. وسم JSX مكسور ومموّه بتعليق نصي خادع
        (
            r#"
            export const Card = () => {
                return (
                    <div className="card">
                        {/* </div> */}
                        <span>Content</span>
                );
            };
            "#,
            "JSX Tag Balance: وسم غير مغلق ومموّه بتعليق",
        ),
        // 9. استدعاء .unwrap() غير مؤمن في كود Rust
        (
            r#"
            pub fn parse_port(port_str: &str) -> u16 {
                port_str.parse::<u16>().unwrap()
            }
            "#,
            "Unsafe Panic: استخدام .unwrap() بدون معالجة خطأ",
        ),
        // 10. عدم توازن الأقواس داخل قالب نصي متداخل
        (
            r#"
            export function buildQuery(id: string) {
                const query = `query { user(id: "${id}" { name }`;
                return query;
            }
            "#,
            "Delimiter Balance: خلل في توازن الأقواس",
        ),
    ];

    for (i, (code, description)) in cases.iter().enumerate() {
        let report = AstGuard::verify(code);
        assert!(
            !report.passed,
            "فشل في رصد الاختبار رقم [{}]: {}",
            i + 1,
            description
        );
        println!(
            "✅ [تم الاصطياد بنجاح] اختبار #{}: {} (السبب: {})",
            i + 1,
            description,
            report
                .violations
                .first()
                .map(|v| v.as_str())
                .unwrap_or("Unknown")
        );
    }
}
